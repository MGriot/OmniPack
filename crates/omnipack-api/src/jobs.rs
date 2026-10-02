//! Asynchronous jobs: long searches run in the background; the caller polls
//! `/jobs/{id}` or gets the result POSTed to a callback URL.

use crate::engine::{ApiError, Engine};
use crate::erp::ErpRequest;
use omnipack_core::PackRequest;
use omnipack_opt::OptimizeOptions;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Semaphore;

/// What to run.
#[derive(Debug, Clone, Deserialize)]
pub struct JobSpec {
    /// `pack` (one pass) or `best` (search). Simple-format requests follow
    /// their `options.fill` unless this says `best`.
    #[serde(default)]
    pub kind: Option<String>,
    /// A full OmniPack request ...
    #[serde(default)]
    pub request: Option<PackRequest>,
    /// ... or a simple-format one.
    #[serde(default)]
    pub erp: Option<ErpRequest>,
    /// Search time for `best`, seconds.
    #[serde(default)]
    pub budget_s: Option<f64>,
    /// Search settings for a full request with `kind: best`.
    #[serde(default)]
    pub options: Option<OptimizeOptions>,
    #[serde(default)]
    pub callback: Option<Callback>,
    #[serde(default)]
    pub reference: Option<String>,
}

/// Where to POST the finished job, with any headers that endpoint needs
/// (for example `Authorization` for SAP Integration Suite).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Callback {
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CallbackStatus {
    pub url: String,
    pub delivered: bool,
    pub attempts: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// A job as the API shows it.
#[derive(Debug, Clone, Serialize)]
pub struct JobView {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    pub kind: String,
    pub status: JobStatus,
    /// Seconds since 1970.
    pub created: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished: Option<u64>,
    /// Search progress: phase, plans evaluated, best score so far.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback: Option<CallbackStatus>,
    /// The plan: a pack result, a search result, or a simple-format answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

struct Entry {
    view: JobView,
    cancel: Arc<AtomicBool>,
    progress: Arc<Mutex<Option<Value>>>,
}

/// The job table: bounded, finished jobs kept for a while.
pub struct Jobs {
    engine: Arc<Engine>,
    table: Mutex<HashMap<String, Entry>>,
    slots: Arc<Semaphore>,
    keep: Duration,
    allow_callbacks: bool,
}

/// Most jobs kept in memory (finished ones are dropped first).
const MAX_STORED: usize = 1000;

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

impl Jobs {
    pub fn new(engine: Arc<Engine>, max_running: usize, keep: Duration, allow_callbacks: bool) -> Arc<Self> {
        Arc::new(Jobs { engine, table: Mutex::new(HashMap::new()), slots: Arc::new(Semaphore::new(max_running.max(1))), keep, allow_callbacks })
    }

    fn purge(table: &mut HashMap<String, Entry>, keep: Duration) {
        let cutoff = now().saturating_sub(keep.as_secs());
        table.retain(|_, e| e.view.finished.is_none_or(|f| f >= cutoff));
        if table.len() > MAX_STORED {
            let mut done: Vec<(u64, String)> = table.iter().filter_map(|(id, e)| e.view.finished.map(|f| (f, id.clone()))).collect();
            done.sort();
            for (_, id) in done.into_iter().take(table.len() - MAX_STORED) {
                table.remove(&id);
            }
        }
    }

    /// Starts a job and returns it (queued).
    pub fn create(self: &Arc<Self>, spec: JobSpec) -> Result<JobView, ApiError> {
        if spec.request.is_none() == spec.erp.is_none() {
            return Err(ApiError::bad("give either `request` (full format) or `erp` (simple format)"));
        }
        if spec.callback.is_some() && !self.allow_callbacks {
            return Err(ApiError::bad("callbacks are switched off on this server"));
        }
        let best = match spec.kind.as_deref().unwrap_or("pack") {
            "pack" => false,
            "best" => true,
            other => return Err(ApiError::bad(format!("unknown kind `{other}` (pack, best)"))),
        };
        let id = format!("{:016x}", rand::thread_rng().gen::<u64>());
        let reference = spec.reference.clone().or_else(|| spec.erp.as_ref().and_then(|e| e.reference.clone()));
        let view = JobView {
            id: id.clone(),
            reference,
            kind: if best { "best" } else { "pack" }.into(),
            status: JobStatus::Queued,
            created: now(),
            finished: None,
            progress: None,
            error: None,
            callback: spec.callback.as_ref().map(|c| CallbackStatus { url: c.url.clone(), ..Default::default() }),
            result: None,
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(Mutex::new(None));
        {
            let mut t = self.table.lock().unwrap();
            Self::purge(&mut t, self.keep);
            t.insert(id.clone(), Entry { view: view.clone(), cancel: cancel.clone(), progress: progress.clone() });
        }
        let jobs = self.clone();
        tokio::spawn(async move { jobs.run(id, spec, best, cancel, progress).await });
        Ok(view)
    }

    async fn run(self: Arc<Self>, id: String, spec: JobSpec, best: bool, cancel: Arc<AtomicBool>, progress: Arc<Mutex<Option<Value>>>) {
        let permit = self.slots.clone().acquire_owned().await;
        if cancel.load(Ordering::Relaxed) {
            self.finish(&id, JobStatus::Cancelled, None, None);
            return;
        }
        self.update(&id, |v| v.status = JobStatus::Running);
        let engine = self.engine.clone();
        let (c, p) = (cancel.clone(), progress.clone());
        let callback = spec.callback.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            let mut report = |pr: &omnipack_opt::Progress| *p.lock().unwrap() = serde_json::to_value(pr).ok();
            match (spec.request, spec.erp) {
                (Some(req), _) if best => {
                    let mut o = spec.options.unwrap_or_default();
                    o.budget_ms = (spec.budget_s.unwrap_or(15.0) * 1000.0) as u64;
                    engine.optimize(req, o, f64::INFINITY, &c, &mut report).and_then(|r| serde_json::to_value(r).map_err(|e| ApiError::internal(e.to_string())))
                }
                (Some(req), _) => engine.pack(req).and_then(|r| serde_json::to_value(r).map_err(|e| ApiError::internal(e.to_string()))),
                (None, Some(mut erp)) => {
                    if best {
                        erp.options.fill = Some("best".into());
                        erp.options.search_seconds = spec.budget_s.or(erp.options.search_seconds);
                    }
                    engine.erp(&erp, f64::INFINITY, &c, &mut report).and_then(|(r, _)| serde_json::to_value(r).map_err(|e| ApiError::internal(e.to_string())))
                }
                (None, None) => Err(ApiError::bad("nothing to run")),
            }
        })
        .await;
        drop(permit);
        let (status, result, error) = match outcome {
            Ok(Ok(v)) => (if cancel.load(Ordering::Relaxed) { JobStatus::Cancelled } else { JobStatus::Done }, Some(v), None),
            Ok(Err(e)) => (JobStatus::Failed, None, Some(e.error)),
            Err(e) => (JobStatus::Failed, None, Some(format!("the engine stopped: {e}"))),
        };
        let view = self.finish(&id, status, result, error);
        if let (Some(cb), Some(view)) = (callback, view) {
            self.deliver(&id, cb, view).await;
        }
    }

    fn update(&self, id: &str, f: impl FnOnce(&mut JobView)) {
        if let Some(e) = self.table.lock().unwrap().get_mut(id) {
            f(&mut e.view);
        }
    }

    fn finish(&self, id: &str, status: JobStatus, result: Option<Value>, error: Option<String>) -> Option<JobView> {
        let mut t = self.table.lock().unwrap();
        let e = t.get_mut(id)?;
        e.view.status = status;
        e.view.finished = Some(now());
        e.view.result = result;
        e.view.error = error;
        e.view.progress = e.progress.lock().unwrap().clone();
        Some(e.view.clone())
    }

    /// POSTs the finished job; three attempts with a growing pause.
    async fn deliver(&self, id: &str, cb: Callback, view: JobView) {
        let client = match reqwest::Client::builder().timeout(Duration::from_secs(30)).build() {
            Ok(c) => c,
            Err(e) => return self.update(id, |v| v.callback.as_mut().unwrap().last_error = Some(e.to_string())),
        };
        let body = serde_json::json!({ "job_id": view.id, "reference": view.reference, "status": view.status, "error": view.error, "result": view.result });
        for attempt in 1..=3u32 {
            let mut req = client.post(&cb.url).json(&body);
            for (k, v) in &cb.headers {
                req = req.header(k, v);
            }
            let outcome = match req.send().await {
                Ok(r) if r.status().is_success() => Ok(()),
                Ok(r) => Err(format!("HTTP {}", r.status())),
                Err(e) => Err(e.to_string()),
            };
            let ok = outcome.is_ok();
            self.update(id, |v| {
                let c = v.callback.as_mut().unwrap();
                c.attempts = attempt;
                c.delivered = ok;
                c.last_error = outcome.err();
            });
            if ok {
                return;
            }
            tokio::time::sleep(Duration::from_secs(u64::from(attempt * attempt))).await;
        }
    }

    /// The job, with its result once finished (or its progress while running).
    pub fn get(&self, id: &str) -> Option<JobView> {
        let t = self.table.lock().unwrap();
        let e = t.get(id)?;
        let mut v = e.view.clone();
        if v.status == JobStatus::Running {
            v.progress = e.progress.lock().unwrap().clone();
        }
        Some(v)
    }

    /// All jobs, newest first, without their results.
    pub fn list(&self) -> Vec<JobView> {
        let t = self.table.lock().unwrap();
        let mut all: Vec<JobView> = t.values().map(|e| JobView { result: None, ..e.view.clone() }).collect();
        all.sort_by(|a, b| b.created.cmp(&a.created).then(b.id.cmp(&a.id)));
        all
    }

    /// Asks a job to stop. A search keeps the best plan found so far.
    pub fn cancel(&self, id: &str) -> Option<JobView> {
        let t = self.table.lock().unwrap();
        let e = t.get(id)?;
        e.cancel.store(true, Ordering::Relaxed);
        Some(JobView { result: None, ..e.view.clone() })
    }

    /// Jobs not finished yet.
    pub fn active(&self) -> usize {
        self.table.lock().unwrap().values().filter(|e| e.view.finished.is_none()).count()
    }
}
