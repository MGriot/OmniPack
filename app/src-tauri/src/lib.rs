//! Tauri shell: exposes the packing engine and a small file-based catalog to
//! the web UI. All computation runs in-process; on desktop the same engine can
//! also be offered to other programs over HTTP (`local_api`).

mod local_api;

use omnipack_core::learn::{self, TrainReport};
use omnipack_core::manual::{self, Probe};
use omnipack_core::{
    generate, pack, pack_with_fixed, ContainerPlan, ContainerSpec, ItemSpec, PackRequest, PackResult, Placement, Ranker, RoadVehicle, SavedPlan, ShipCase, ShipMotion,
    TransportCase,
};
use omnipack_geom::{Orientation, OrientedShape, RenderMesh, Shape};
use omnipack_opt::{optimize, OptimizeOptions, OptimizeResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
async fn pack_request(request: PackRequest) -> CmdResult<PackResult> {
    tauri::async_runtime::spawn_blocking(move || pack(&request).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

/// Set by `cancel_optimize`; the running search returns its best plans so far.
static CANCEL: AtomicBool = AtomicBool::new(false);

/// Searches fill patterns, loading orders and orientations for the best plans
/// (see `omnipack_opt`). Emits `optimize-progress` events while it runs.
#[tauri::command]
async fn optimize_request(app: AppHandle, request: PackRequest, options: OptimizeOptions) -> CmdResult<OptimizeResult> {
    CANCEL.store(false, Ordering::Relaxed);
    tauri::async_runtime::spawn_blocking(move || {
        let mut last = Instant::now();
        optimize(&request, &options, &CANCEL, &mut |p| {
            if last.elapsed() >= Duration::from_millis(150) {
                last = Instant::now();
                let _ = app.emit("optimize-progress", p);
            }
        })
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn cancel_optimize() {
    CANCEL.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn sample_request(kind: String, seed: u64) -> CmdResult<PackRequest> {
    match kind.as_str() {
        "mixed" => Ok(generate::mixed(seed)),
        "shapes" => Ok(generate::shapes(seed)),
        k if k.starts_with("br") => {
            let class = k[2..].parse().map_err(|_| format!("bad sample `{k}`"))?;
            generate::br_like(class, seed).ok_or_else(|| format!("bad sample `{k}`"))
        }
        _ => Err(format!("unknown sample `{kind}`")),
    }
}

/// Built-in transport acceleration profiles (road, rail, sea).
#[tauri::command]
fn transport_presets() -> Vec<TransportCase> {
    TransportCase::presets()
}

/// Typical ISO containers and a road semi-trailer.
#[tauri::command]
fn container_presets() -> Vec<ContainerSpec> {
    ContainerSpec::presets()
}

/// Tractor + semi-trailer combinations for the axle-load check.
#[tauri::command]
fn vehicle_presets() -> Vec<RoadVehicle> {
    RoadVehicle::presets()
}

/// A sea transport case from the ship's roll and pitch at a stowage position.
#[tauri::command]
fn ship_motion_case(motion: ShipMotion) -> ShipCase {
    TransportCase::from_ship(&motion)
}

/// Triangle mesh of a shape in one orientation, centred on its bounding box,
/// so the viewer draws exactly the geometry the physics uses.
#[tauri::command]
fn shape_mesh(shape: Shape, orientation: Orientation) -> CmdResult<RenderMesh> {
    if !shape.is_valid() {
        return Err(format!("invalid shape {shape:?}"));
    }
    Ok(OrientedShape::new(&shape, orientation, [0.0; 3]).render_mesh())
}

/// One item on its own, unrotated, for the preview in its editor card.
#[derive(Serialize)]
struct ItemPreview {
    mesh: RenderMesh,
    /// Bounding box W, H, D, mm.
    extents: [f64; 3],
    /// Uniform-density centre of mass, from the bounding box's min corner.
    centroid: [f64; 3],
    /// Centre of mass including `com_offset`, from the min corner.
    com: [f64; 3],
}

#[tauri::command]
fn item_preview(shape: Shape, com_offset: [f64; 3]) -> CmdResult<ItemPreview> {
    if !shape.is_valid() {
        return Err(format!("invalid shape {shape:?}"));
    }
    let plain = OrientedShape::new(&shape, Orientation::Whd, [0.0; 3]);
    let com = OrientedShape::new(&shape, Orientation::Whd, com_offset).com_from_min;
    Ok(ItemPreview { mesh: plain.render_mesh(), extents: plain.extents, centroid: plain.com_from_min, com })
}

/// One allowed resting orientation of an item and its bounding box there.
#[derive(Serialize)]
struct Pose {
    orientation: Orientation,
    extents: [f64; 3],
}

/// The orientations an item may be placed in (as the placer uses them).
#[tauri::command]
fn item_poses(item: ItemSpec, allow_rotation: bool) -> CmdResult<Vec<Pose>> {
    if !item.shape.is_valid() {
        return Err(format!("invalid shape {:?}", item.shape));
    }
    Ok(item
        .orientations(allow_rotation)
        .into_iter()
        .map(|o| Pose { orientation: o, extents: OrientedShape::new(&item.shape, o, item.com_offset).extents })
        .collect())
}

/// Saved setups, keyed by name. Stored as one JSON file in the app data dir.
#[derive(Default, Serialize, Deserialize)]
struct Catalog {
    entries: BTreeMap<String, PackRequest>,
}

fn catalog_path(app: &AppHandle) -> CmdResult<PathBuf> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("catalog.json"))
}

fn read_catalog(app: &AppHandle) -> CmdResult<Catalog> {
    let path = catalog_path(app)?;
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Catalog::default()),
        Err(e) => Err(e.to_string()),
    }
}

fn write_catalog(app: &AppHandle, catalog: &Catalog) -> CmdResult<()> {
    let path = catalog_path(app)?;
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(catalog).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

#[tauri::command]
fn catalog_list(app: AppHandle) -> CmdResult<Vec<String>> {
    Ok(read_catalog(&app)?.entries.into_keys().collect())
}

#[tauri::command]
fn catalog_load(app: AppHandle, name: String) -> CmdResult<PackRequest> {
    read_catalog(&app)?.entries.remove(&name).ok_or_else(|| format!("`{name}` not found"))
}

#[tauri::command]
fn catalog_save(app: AppHandle, name: String, request: PackRequest) -> CmdResult<()> {
    let mut c = read_catalog(&app)?;
    c.entries.insert(name, request);
    write_catalog(&app, &c)
}

#[tauri::command]
fn catalog_delete(app: AppHandle, name: String) -> CmdResult<()> {
    let mut c = read_catalog(&app)?;
    c.entries.remove(&name);
    write_catalog(&app, &c)
}

// ---------- manual placement ----------

/// The hand-made plan being edited, kept here so pointer probes need not
/// resend it.
#[derive(Default)]
struct ManualSession {
    req: Option<PackRequest>,
    placements: Vec<Placement>,
}

#[derive(Serialize)]
struct ManualView {
    plan: ContainerPlan,
    /// Units of each item still to place.
    remaining: Vec<(String, u32)>,
}

type Session<'a> = State<'a, Mutex<ManualSession>>;

fn manual_view(s: &mut ManualSession) -> CmdResult<ManualView> {
    let req = s.req.as_ref().ok_or("no manual plan started")?;
    let plan = manual::evaluate(req, std::mem::take(&mut s.placements));
    s.placements = plan.placements.clone();
    Ok(ManualView { remaining: manual::remaining(req, &s.placements), plan })
}

/// Starts (or restores, for undo) a hand-made plan.
#[tauri::command]
fn manual_set(session: Session, request: PackRequest, placements: Vec<Placement>) -> CmdResult<ManualView> {
    let mut s = session.lock().map_err(|e| e.to_string())?;
    s.req = Some(request);
    s.placements = placements;
    manual_view(&mut s)
}

/// Where a unit would land at the pointer, and what would be wrong there.
#[tauri::command]
fn manual_probe(session: Session, item_id: String, orientation: Orientation, x: f64, z: f64, y: Option<f64>, replace: Option<String>) -> CmdResult<Probe> {
    let s = session.lock().map_err(|e| e.to_string())?;
    let req = s.req.as_ref().ok_or("no manual plan started")?;
    manual::probe(req, &s.placements, &item_id, orientation, x, z, y, replace.as_deref())
}

/// Adds a unit, or moves `replace` to the new pose.
#[tauri::command]
fn manual_place(session: Session, item_id: String, orientation: Orientation, x: f64, z: f64, y: Option<f64>, replace: Option<String>) -> CmdResult<ManualView> {
    let mut s = session.lock().map_err(|e| e.to_string())?;
    let req = s.req.as_ref().ok_or("no manual plan started")?;
    let p = manual::place(req, &s.placements, &item_id, orientation, x, z, y, replace.as_deref())?;
    s.placements.retain(|q| Some(q.instance_id.as_str()) != replace.as_deref());
    s.placements.push(p);
    manual_view(&mut s)
}

#[tauri::command]
fn manual_remove(session: Session, instance_id: String) -> CmdResult<ManualView> {
    let mut s = session.lock().map_err(|e| e.to_string())?;
    s.placements.retain(|q| q.instance_id != instance_id);
    manual_view(&mut s)
}

/// Packs the remaining units around the hand-placed ones.
#[tauri::command]
async fn manual_auto_fill(session: Session<'_>) -> CmdResult<PackResult> {
    let (req, fixed) = {
        let s = session.lock().map_err(|e| e.to_string())?;
        (s.req.clone().ok_or("no manual plan started")?, s.placements.clone())
    };
    tauri::async_runtime::spawn_blocking(move || pack_with_fixed(&req, &fixed).map_err(|e| e.to_string())).await.map_err(|e| e.to_string())?
}

// ---------- saved solutions and the learned ranker ----------

#[derive(Clone, Serialize, Deserialize)]
struct SolutionMeta {
    id: String,
    name: String,
    created: u64,
    source: String,
    train: bool,
    valid: bool,
    summary: String,
}

fn data_dir(app: &AppHandle, sub: &str) -> CmdResult<PathBuf> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join(sub);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Writes through a temporary file, so a crash never leaves half a file.
fn write_atomic(path: &std::path::Path, text: &str) -> CmdResult<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn read_index(app: &AppHandle) -> CmdResult<Vec<SolutionMeta>> {
    let path = data_dir(app, "solutions")?.join("index.json");
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}

fn write_index(app: &AppHandle, index: &[SolutionMeta]) -> CmdResult<()> {
    let path = data_dir(app, "solutions")?.join("index.json");
    write_atomic(&path, &serde_json::to_string_pretty(index).map_err(|e| e.to_string())?)
}

fn solution_path(app: &AppHandle, id: &str) -> CmdResult<PathBuf> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(format!("bad solution id `{id}`"));
    }
    Ok(data_dir(app, "solutions")?.join(format!("{id}.json")))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Saves a plan with its setup. Only a valid plan can be used for training.
#[tauri::command]
fn solution_save(app: AppHandle, name: String, source: String, train: bool, request: PackRequest, result: PackResult) -> CmdResult<SolutionMeta> {
    let created = now_secs();
    let mut index = read_index(&app)?;
    let mut id = format!("s{created}");
    let mut k = 1;
    while index.iter().any(|m| m.id == id) {
        k += 1;
        id = format!("s{created}-{k}");
    }
    let valid = result.is_valid();
    let summary = format!(
        "{}/{} units, {} container(s), {:.1}% vol",
        result.packed_units,
        result.requested_units,
        result.containers.len(),
        result.volume_utilization * 100.0
    );
    let saved = SavedPlan { id: id.clone(), name: name.clone(), created, source: source.clone(), train: train && valid, request, result };
    write_atomic(&solution_path(&app, &id)?, &serde_json::to_string(&saved).map_err(|e| e.to_string())?)?;
    let meta = SolutionMeta { id, name, created, source, train: train && valid, valid, summary };
    index.push(meta.clone());
    write_index(&app, &index)?;
    Ok(meta)
}

#[tauri::command]
fn solution_list(app: AppHandle) -> CmdResult<Vec<SolutionMeta>> {
    read_index(&app)
}

#[tauri::command]
fn solution_load(app: AppHandle, id: String) -> CmdResult<SavedPlan> {
    let path = solution_path(&app, &id)?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

#[tauri::command]
fn solution_set_train(app: AppHandle, id: String, train: bool) -> CmdResult<Vec<SolutionMeta>> {
    let mut index = read_index(&app)?;
    let m = index.iter_mut().find(|m| m.id == id).ok_or_else(|| format!("`{id}` not found"))?;
    if train && !m.valid {
        return Err("A plan with violations cannot be used for training.".into());
    }
    m.train = train;
    write_index(&app, &index)?;
    Ok(index)
}

#[tauri::command]
fn solution_delete(app: AppHandle, id: String) -> CmdResult<Vec<SolutionMeta>> {
    let mut index = read_index(&app)?;
    index.retain(|m| m.id != id);
    let _ = std::fs::remove_file(solution_path(&app, &id)?);
    write_index(&app, &index)?;
    Ok(index)
}

/// The learned ranker and how it was trained.
#[derive(Clone, Serialize, Deserialize)]
struct ModelFile {
    ranker: Ranker,
    report: TrainReport,
    trained: u64,
}

fn model_path(app: &AppHandle) -> CmdResult<PathBuf> {
    Ok(data_dir(app, "")?.join("model.json"))
}

/// The flagged, valid saved plans, re-checked from their placements.
fn training_plans(app: &AppHandle) -> CmdResult<Vec<(PackRequest, Vec<ContainerPlan>)>> {
    let mut plans = Vec::new();
    for m in read_index(app)?.into_iter().filter(|m| m.train && m.valid) {
        let mut saved = solution_load(app.clone(), m.id)?;
        learn::revalidate(&saved.request, &mut saved.result.containers);
        if learn::trainable(&saved.result.containers) {
            plans.push((saved.request, saved.result.containers));
        }
    }
    Ok(plans)
}

#[tauri::command]
async fn model_train(app: AppHandle) -> CmdResult<ModelFile> {
    tauri::async_runtime::spawn_blocking(move || {
        let plans = training_plans(&app)?;
        if plans.is_empty() {
            return Err("No saved plan is marked for training yet (only valid plans can be).".to_string());
        }
        let (ranker, report) = learn::train_on(&plans).ok_or("The marked plans contain no placement decisions to learn from.")?;
        let model = ModelFile { ranker, report, trained: now_secs() };
        write_atomic(&model_path(&app)?, &serde_json::to_string_pretty(&model).map_err(|e| e.to_string())?)?;
        Ok((model, app))
    })
    .await
    .map_err(|e| e.to_string())?
    .map(|(model, app)| {
        // A running local API switches to the new model.
        tauri::async_runtime::spawn(async move { local_api::reload(&app).await });
        model
    })
}

#[tauri::command]
fn model_info(app: AppHandle) -> CmdResult<Option<ModelFile>> {
    match std::fs::read_to_string(model_path(&app)?) {
        Ok(text) => Ok(serde_json::from_str(&text).ok()),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
fn model_reset(app: AppHandle) -> CmdResult<()> {
    let _ = std::fs::remove_file(model_path(&app)?);
    Ok(())
}

/// The placement decisions of the flagged plans as JSON lines, for training
/// other models (see docs/learning.md).
#[tauri::command]
async fn training_data(app: AppHandle) -> CmdResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let plans = training_plans(&app)?;
        let steps: Vec<learn::Step> = plans.iter().flat_map(|(req, cs)| cs.iter().flat_map(move |c| learn::examples(req, c))).collect();
        Ok(learn::export_jsonl(&steps))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // File access only to paths the user picks in a dialog.
        .plugin(tauri_plugin_fs::init())
        .manage(Mutex::new(ManualSession::default()))
        .manage(local_api::LocalApi::default())
        .setup(|app| {
            local_api::autostart(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pack_request,
            optimize_request,
            cancel_optimize,
            sample_request,
            transport_presets,
            container_presets,
            vehicle_presets,
            ship_motion_case,
            shape_mesh,
            item_preview,
            item_poses,
            catalog_list,
            catalog_load,
            catalog_save,
            catalog_delete,
            manual_set,
            manual_probe,
            manual_place,
            manual_remove,
            manual_auto_fill,
            solution_save,
            solution_list,
            solution_load,
            solution_set_train,
            solution_delete,
            model_train,
            model_info,
            model_reset,
            training_data,
            local_api::imp::api_status,
            local_api::imp::api_apply,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OmniPack");
}
