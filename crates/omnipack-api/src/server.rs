//! The HTTP server: routes, API keys, limits, optional HTTPS, and the drop
//! folder watcher. The same router serves the standalone `omnipack-server`
//! and the desktop app's local API.

use crate::csv;
use crate::drop::{self, DropConfig};
use crate::engine::{ApiError, Engine};
use crate::erp::{self, ErpRequest};
use crate::jobs::{JobSpec, Jobs};
use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use omnipack_core::manual;
use omnipack_core::{ContainerSpec, PackRequest, PackResult, Placement, Ranker, RoadVehicle, TransportCase, PLAN_SCHEMA};
use omnipack_geom::Orientation;
use omnipack_opt::{OptimizeOptions, OptimizeResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::cors::{Any, CorsLayer};

/// The OpenAPI 3.1 description of this API.
pub const OPENAPI: &str = include_str!("../openapi.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// Address to listen on, e.g. `127.0.0.1:8765` or `0.0.0.0:8765`.
    pub bind: String,
    /// Accepted keys (`X-API-Key` or `Authorization: Bearer`). Required
    /// unless the server only listens on this computer.
    pub api_keys: Vec<String>,
    /// Listen on the network without a key (trusted networks only).
    pub insecure_no_auth: bool,
    /// Origins allowed to call the API from a browser (`*` = any).
    pub cors: Vec<String>,
    /// HTTPS certificate and key (PEM). Without them the server speaks HTTP.
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub max_body_mb: usize,
    pub max_units: usize,
    /// Longest search a synchronous request may run, seconds.
    pub max_sync_seconds: f64,
    /// Longest search a job or drop file may run, seconds.
    pub max_job_seconds: f64,
    /// Jobs running at the same time (the rest wait).
    pub max_jobs: usize,
    pub job_ttl_hours: u64,
    pub allow_callbacks: bool,
    /// One line per request on stderr.
    pub log: bool,
    /// Learned placement model (`omnipack train`), for `fill: learned`.
    pub ranker_file: Option<PathBuf>,
    #[serde(skip)]
    pub ranker: Option<Ranker>,
    pub drop: Option<DropConfig>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig {
            bind: "127.0.0.1:8765".into(),
            api_keys: Vec::new(),
            insecure_no_auth: false,
            cors: Vec::new(),
            tls_cert: None,
            tls_key: None,
            max_body_mb: 10,
            max_units: 20_000,
            max_sync_seconds: 60.0,
            max_job_seconds: 600.0,
            max_jobs: 2,
            job_ttl_hours: 24,
            allow_callbacks: true,
            log: true,
            ranker_file: None,
            ranker: None,
            drop: None,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<ApiConfig>,
    pub engine: Arc<Engine>,
    pub jobs: Arc<Jobs>,
}

impl AppState {
    pub fn new(cfg: ApiConfig) -> Self {
        let engine = Arc::new(Engine { ranker: cfg.ranker.clone(), max_search_s: cfg.max_job_seconds, max_units: cfg.max_units });
        let jobs = Jobs::new(engine.clone(), cfg.max_jobs, Duration::from_secs(cfg.job_ttl_hours * 3600), cfg.allow_callbacks);
        AppState { cfg: Arc::new(cfg), engine, jobs }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(self)).into_response()
    }
}

fn body<T>(r: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    r.map(|Json(v)| v).map_err(|e| ApiError::bad(e.body_text()))
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, ApiError> + Send + 'static) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(f).await.map_err(|e| ApiError::internal(format!("the engine stopped: {e}")))?
}

#[derive(Debug, Default, Deserialize)]
struct FormatQuery {
    format: Option<String>,
}

fn wants_csv(headers: &HeaderMap, q: &FormatQuery) -> bool {
    q.format.as_deref().is_some_and(|f| f.eq_ignore_ascii_case("csv"))
        || headers.get(header::ACCEPT).and_then(|a| a.to_str().ok()).is_some_and(|a| a.contains("text/csv"))
}

fn csv_response(text: String) -> Response {
    ([(header::CONTENT_TYPE, "text/csv; charset=utf-8")], text).into_response()
}

async fn index() -> Json<Value> {
    Json(json!({
        "service": "OmniPack API",
        "version": env!("CARGO_PKG_VERSION"),
        "spec": "/api/v1/openapi.json",
        "docs": "https://github.com/MGriot/OmniPack/blob/main/docs/api.md",
    }))
}

async fn health(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "jobs_active": s.jobs.active(),
        "learned_model": s.engine.ranker.is_some(),
    }))
}

async fn presets() -> Json<Value> {
    let keys = ["road", "rail", "rail_shunting", "sea_a", "sea_b", "sea_c"];
    let transport: Vec<Value> = keys.iter().zip(TransportCase::presets()).map(|(k, c)| json!({ "key": k, "case": c })).collect();
    Json(json!({
        "containers": ContainerSpec::presets(),
        "vehicles": RoadVehicle::presets(),
        "transport": transport,
    }))
}

async fn openapi() -> Response {
    ([(header::CONTENT_TYPE, "application/json")], OPENAPI).into_response()
}

async fn pack_full(State(s): State<AppState>, headers: HeaderMap, Query(q): Query<FormatQuery>, payload: Result<Json<PackRequest>, JsonRejection>) -> Result<Response, ApiError> {
    let req = body(payload)?;
    let e = s.engine.clone();
    let r = blocking(move || e.pack(req)).await?;
    Ok(if wants_csv(&headers, &q) { csv_response(csv::load_list(&r)) } else { Json(r).into_response() })
}

#[derive(Deserialize)]
struct OptimizeBody {
    request: PackRequest,
    #[serde(default)]
    options: Option<OptimizeOptions>,
    /// Search time, seconds (default 15; capped by the server).
    #[serde(default)]
    budget_s: Option<f64>,
}

async fn optimize_full(State(s): State<AppState>, payload: Result<Json<OptimizeBody>, JsonRejection>) -> Result<Json<OptimizeResult>, ApiError> {
    let b = body(payload)?;
    let (e, max_s) = (s.engine.clone(), s.cfg.max_sync_seconds);
    let mut o = b.options.unwrap_or_default();
    o.budget_ms = (b.budget_s.unwrap_or(o.budget_ms as f64 / 1000.0) * 1000.0) as u64;
    Ok(Json(blocking(move || e.optimize(b.request, o, max_s, &AtomicBool::new(false), &mut |_| {})).await?))
}

async fn erp_plan(State(s): State<AppState>, headers: HeaderMap, Query(q): Query<FormatQuery>, payload: Result<Json<ErpRequest>, JsonRejection>) -> Result<Response, ApiError> {
    let req = body(payload)?;
    let (e, max_s) = (s.engine.clone(), s.cfg.max_sync_seconds);
    let (resp, result) = blocking(move || e.erp(&req, max_s, &AtomicBool::new(false), &mut |_| {})).await?;
    Ok(if wants_csv(&headers, &q) { csv_response(csv::load_list(&result)) } else { Json(resp).into_response() })
}

/// A placement to check in the full format: mm, AABB minimum corner.
#[derive(Deserialize)]
struct PlacementIn {
    item_id: String,
    #[serde(default)]
    orientation: Option<Orientation>,
    position: [f64; 3],
}

#[derive(Deserialize)]
struct ValidateFull {
    request: PackRequest,
    placements: Vec<PlacementIn>,
}

/// Places the given units exactly where they are said to be (no gravity)
/// and checks the plan.
fn check_given(req: &PackRequest, given: impl IntoIterator<Item = (String, Orientation, [f64; 3])>, max_units: usize) -> Result<omnipack_core::ContainerPlan, ApiError> {
    let mut placed: Vec<Placement> = Vec::new();
    for (k, (item, o, [x, y, z])) in given.into_iter().enumerate() {
        if k >= max_units {
            return Err(ApiError::too_large(format!("more than {max_units} placements")));
        }
        let p = manual::place(req, &placed, &item, o, x, z, Some(y), None).map_err(ApiError::bad)?;
        placed.push(p);
    }
    Ok(manual::evaluate(req, placed))
}

async fn validate(State(s): State<AppState>, payload: Result<Json<Value>, JsonRejection>) -> Result<Response, ApiError> {
    let v = body(payload)?;
    let (max_units, max_s) = (s.cfg.max_units, s.cfg.max_sync_seconds);
    let ranker = s.engine.ranker.clone();
    if v.get("request").is_some() {
        let b: ValidateFull = serde_json::from_value(v).map_err(|e| ApiError::bad(e.to_string()))?;
        let plan = blocking(move || {
            let given = b.placements.into_iter().map(|p| (p.item_id, p.orientation.unwrap_or(Orientation::Whd), p.position));
            check_given(&b.request, given, max_units)
        })
        .await?;
        return Ok(Json(plan).into_response());
    }
    let erp: ErpRequest = serde_json::from_value(v).map_err(|e| ApiError::bad(e.to_string()))?;
    let resp = blocking(move || {
        let given_in = erp.placements.clone().ok_or_else(|| ApiError::bad("`placements` is required to validate"))?;
        let plan = erp.to_plan(max_s, ranker.as_ref()).map_err(ApiError::bad)?;
        let mut given = Vec::with_capacity(given_in.len());
        for g in given_in {
            let o = match g.rotation.as_deref() {
                None | Some("") => Orientation::Whd,
                Some(r) => serde_json::from_value(Value::String(r.to_ascii_uppercase())).map_err(|_| ApiError::bad(format!("unknown rotation `{r}` (WHD, DHW, HWD, WDH, HDW, DWH)")))?,
            };
            given.push((g.item_id, o, [g.x * plan.mm, g.y * plan.mm, g.z * plan.mm]));
        }
        let c = check_given(&plan.request, given, max_units)?;
        let result = PackResult {
            schema: PLAN_SCHEMA.into(),
            volume_utilization: c.metrics.volume_utilization,
            requested_units: plan.request.items.iter().map(|i| i.quantity as usize).sum(),
            packed_units: c.placements.len(),
            containers: vec![c],
            unpacked: Vec::new(),
            elapsed_ms: 0,
        };
        erp::response(erp.reference.clone(), &erp.units, &result).map_err(ApiError::bad)
    })
    .await?;
    Ok(Json(resp).into_response())
}

async fn job_create(State(s): State<AppState>, payload: Result<Json<JobSpec>, JsonRejection>) -> Result<Response, ApiError> {
    let view = s.jobs.create(body(payload)?)?;
    let location = format!("/api/v1/jobs/{}", view.id);
    Ok((StatusCode::ACCEPTED, [(header::LOCATION, location)], Json(view)).into_response())
}

async fn job_list(State(s): State<AppState>) -> Response {
    Json(s.jobs.list()).into_response()
}

async fn job_get(State(s): State<AppState>, Path(id): Path<String>) -> Result<Response, ApiError> {
    s.jobs.get(&id).map(|v| Json(v).into_response()).ok_or_else(|| ApiError::not_found(format!("no job `{id}`")))
}

async fn job_cancel(State(s): State<AppState>, Path(id): Path<String>) -> Result<Response, ApiError> {
    s.jobs.cancel(&id).map(|v| Json(v).into_response()).ok_or_else(|| ApiError::not_found(format!("no job `{id}`")))
}

async fn not_found() -> ApiError {
    ApiError::not_found("no such endpoint; see /api/v1/openapi.json")
}

/// Compares keys in constant time (per length).
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn auth(State(s): State<AppState>, req: Request, next: Next) -> Response {
    let open = matches!(req.uri().path(), "/" | "/api/v1/health" | "/api/v1/openapi.json");
    if s.cfg.api_keys.is_empty() || open || req.method() == Method::OPTIONS {
        return next.run(req).await;
    }
    let h = req.headers();
    let given = h
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| h.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")))
        .map(str::trim);
    if given.is_some_and(|g| s.cfg.api_keys.iter().any(|k| same(k, g))) {
        next.run(req).await
    } else {
        ApiError { status: 401, error: "missing or wrong API key: send it as X-API-Key or Authorization: Bearer".into() }.into_response()
    }
}

async fn access_log(State(s): State<AppState>, req: Request, next: Next) -> Response {
    let (method, path, t0) = (req.method().clone(), req.uri().path().to_string(), Instant::now());
    let resp = next.run(req).await;
    if s.cfg.log {
        eprintln!("{method} {path} {} {} ms", resp.status().as_u16(), t0.elapsed().as_millis());
    }
    resp
}

/// All routes, with authentication, limits and CORS.
pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/presets", get(presets))
        .route("/openapi.json", get(openapi))
        .route("/pack", post(pack_full))
        .route("/optimize", post(optimize_full))
        .route("/erp/plan", post(erp_plan))
        .route("/validate", post(validate))
        .route("/jobs", post(job_create).get(job_list))
        .route("/jobs/{id}", get(job_get).delete(job_cancel));
    let mut app = Router::new()
        .route("/", get(index))
        .nest("/api/v1", api)
        .fallback(not_found)
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .layer(DefaultBodyLimit::max(state.cfg.max_body_mb.max(1) * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), access_log))
        .with_state(state.clone());
    if !state.cfg.cors.is_empty() {
        let cors = CorsLayer::new().allow_methods(Any).allow_headers(Any);
        let cors = if state.cfg.cors.iter().any(|o| o == "*") {
            cors.allow_origin(Any)
        } else {
            cors.allow_origin(state.cfg.cors.iter().filter_map(|o| HeaderValue::from_str(o).ok()).collect::<Vec<_>>())
        };
        app = app.layer(cors);
    }
    app
}

/// A running server.
pub struct ServeHandle {
    /// Base URL, e.g. `http://127.0.0.1:8765`.
    pub url: String,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    tls: Option<axum_server::Handle>,
    drop_stop: Option<tokio::sync::watch::Sender<bool>>,
    task: tokio::task::JoinHandle<()>,
}

impl ServeHandle {
    pub fn is_running(&self) -> bool {
        !self.task.is_finished()
    }

    /// Stops accepting requests, lets running ones finish (up to 5 s).
    pub async fn stop(mut self) {
        if let Some(tx) = self.stop.take() {
            let _ = tx.send(());
        }
        if let Some(h) = self.tls.take() {
            h.graceful_shutdown(Some(Duration::from_secs(5)));
        }
        if let Some(d) = self.drop_stop.take() {
            let _ = d.send(true);
        }
        let _ = tokio::time::timeout(Duration::from_secs(6), &mut self.task).await;
    }
}

/// Starts the server (and the drop folder watcher) in the background.
pub async fn serve(mut cfg: ApiConfig) -> Result<ServeHandle, String> {
    let addr: SocketAddr = tokio::net::lookup_host(&cfg.bind)
        .await
        .map_err(|e| format!("bind address `{}`: {e}", cfg.bind))?
        .next()
        .ok_or_else(|| format!("bind address `{}` does not resolve", cfg.bind))?;
    if cfg.api_keys.is_empty() && !cfg.insecure_no_auth && !addr.ip().is_loopback() {
        return Err(format!("refusing to listen on {addr} without an API key: set one, or listen on 127.0.0.1 only"));
    }
    if cfg.ranker.is_none() {
        if let Some(path) = &cfg.ranker_file {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            // Either a ranker (`omnipack train`) or the app's model.json.
            let v: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            cfg.ranker = Some(serde_json::from_value(v.get("ranker").cloned().unwrap_or(v)).map_err(|e| format!("{}: {e}", path.display()))?);
        }
    }
    let state = AppState::new(cfg);
    let app = router(state.clone());
    let drop_stop = state.cfg.drop.clone().map(|d| {
        let (tx, rx) = tokio::sync::watch::channel(false);
        tokio::spawn(drop::watch(d, state.engine.clone(), rx));
        tx
    });
    match (&state.cfg.tls_cert, &state.cfg.tls_key) {
        (Some(cert), Some(key)) => {
            let _ = rustls::crypto::ring::default_provider().install_default();
            let tls = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await.map_err(|e| format!("TLS certificate/key: {e}"))?;
            let handle = axum_server::Handle::new();
            let server = axum_server::bind_rustls(addr, tls).handle(handle.clone());
            let task = tokio::spawn(async move {
                if let Err(e) = server.serve(app.into_make_service()).await {
                    eprintln!("server stopped: {e}");
                }
            });
            let bound = tokio::time::timeout(Duration::from_secs(5), handle.listening()).await.ok().flatten().unwrap_or(addr);
            Ok(ServeHandle { url: format!("https://{bound}"), stop: None, tls: Some(handle), drop_stop, task })
        }
        (None, None) => {
            let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| format!("cannot listen on {addr}: {e}"))?;
            let bound = listener.local_addr().map_err(|e| e.to_string())?;
            let (tx, rx) = tokio::sync::oneshot::channel::<()>();
            let task = tokio::spawn(async move {
                let shutdown = async move {
                    let _ = rx.await;
                };
                if let Err(e) = axum::serve(listener, app).with_graceful_shutdown(shutdown).await {
                    eprintln!("server stopped: {e}");
                }
            });
            Ok(ServeHandle { url: format!("http://{bound}"), stop: Some(tx), tls: None, drop_stop, task })
        }
        _ => Err("HTTPS needs both a certificate and a key".into()),
    }
}
