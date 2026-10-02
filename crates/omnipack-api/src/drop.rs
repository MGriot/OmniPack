//! File drop folders, for systems that exchange files rather than calling an
//! API (for example an SAP PI/PO file adapter).
//!
//! Every few seconds the inbox is scanned for `*.json` (full or simple
//! request) and `*.csv` (simple-format item list) files. For each one the
//! outbox receives `<name>.result.json` and `<name>.loadlist.csv`, or
//! `<name>.error.txt`; outputs are written to a temporary name first and then
//! renamed, so a reader never sees half a file. The input is then moved to
//! the archive folder.

use crate::csv;
use crate::engine::Engine;
use crate::erp::{ErpContainer, ErpOptions, ErpRequest, Units};
use omnipack_core::PackRequest;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DropConfig {
    pub inbox: PathBuf,
    pub outbox: PathBuf,
    /// Where processed inputs go (default: `<inbox>/archive`).
    pub archive: Option<PathBuf>,
    /// Container, units and options for CSV item lists (JSON files carry their own).
    pub container: ErpContainer,
    pub units: Units,
    pub options: ErpOptions,
    /// Scan interval, ms.
    pub poll_ms: u64,
}

impl Default for DropConfig {
    fn default() -> Self {
        DropConfig {
            inbox: PathBuf::from("inbox"),
            outbox: PathBuf::from("outbox"),
            archive: None,
            container: ErpContainer { preset: Some("40ft-hc".into()), ..Default::default() },
            units: Units::default(),
            options: ErpOptions::default(),
            poll_ms: 2000,
        }
    }
}

impl DropConfig {
    fn archive_dir(&self) -> PathBuf {
        self.archive.clone().unwrap_or_else(|| self.inbox.join("archive"))
    }
}

fn write_atomic(dir: &Path, name: &str, text: &str) -> std::io::Result<()> {
    let tmp = dir.join(format!(".{name}.tmp"));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, dir.join(name))
}

/// The result JSON and load list of one input file.
fn process(path: &Path, cfg: &DropConfig, engine: &Engine) -> Result<(String, String), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("request").to_string();
    let cancel = AtomicBool::new(false);
    let is_csv = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("csv"));
    if !is_csv {
        if let Ok(req) = serde_json::from_str::<PackRequest>(&text) {
            let r = engine.pack(req).map_err(|e| e.error)?;
            return Ok((serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?, csv::load_list(&r)));
        }
    }
    let erp = if is_csv {
        ErpRequest {
            reference: Some(stem),
            units: cfg.units.clone(),
            container: cfg.container.clone(),
            vehicle: None,
            items: csv::parse_items(&text)?,
            options: cfg.options.clone(),
            placements: None,
        }
    } else {
        serde_json::from_str(&text).map_err(|e| format!("neither a full nor a simple-format request: {e}"))?
    };
    let (resp, result) = engine.erp(&erp, f64::INFINITY, &cancel, &mut |_| {}).map_err(|e| e.error)?;
    Ok((serde_json::to_string_pretty(&resp).map_err(|e| e.to_string())?, csv::load_list(&result)))
}

/// Processes every input file in the inbox that is at least `settle` old
/// (younger ones may still be being written). Returns the names handled.
pub fn scan_once(cfg: &DropConfig, engine: &Engine, settle: Duration) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(&cfg.inbox) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json") || e.eq_ignore_ascii_case("csv")))
        .filter(|p| {
            let age = std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|m| SystemTime::now().duration_since(m).ok());
            age.is_none_or(|a| a >= settle)
        })
        .collect();
    files.sort();
    let archive = cfg.archive_dir();
    let _ = std::fs::create_dir_all(&cfg.outbox);
    let _ = std::fs::create_dir_all(&archive);
    let mut done = Vec::new();
    for path in files {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        match process(&path, cfg, engine) {
            Ok((json, list)) => {
                let _ = write_atomic(&cfg.outbox, &format!("{stem}.result.json"), &json);
                let _ = write_atomic(&cfg.outbox, &format!("{stem}.loadlist.csv"), &list);
            }
            Err(e) => {
                let _ = write_atomic(&cfg.outbox, &format!("{stem}.error.txt"), &format!("{name}: {e}\n"));
            }
        }
        // Keep the input, but never process it twice.
        let target = archive.join(&name);
        if std::fs::rename(&path, &target).is_err() {
            let _ = std::fs::copy(&path, &target).and_then(|_| std::fs::remove_file(&path));
        }
        done.push(name);
    }
    done
}

/// Watches the inbox until `stop` turns true.
pub async fn watch(cfg: DropConfig, engine: std::sync::Arc<Engine>, mut stop: tokio::sync::watch::Receiver<bool>) {
    let every = Duration::from_millis(cfg.poll_ms.max(200));
    loop {
        let (c, e) = (cfg.clone(), engine.clone());
        let _ = tokio::task::spawn_blocking(move || scan_once(&c, &e, Duration::from_secs(1))).await;
        tokio::select! {
            _ = tokio::time::sleep(every) => {}
            _ = stop.changed() => return,
        }
        if *stop.borrow() {
            return;
        }
    }
}
