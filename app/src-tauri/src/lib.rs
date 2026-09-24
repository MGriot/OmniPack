//! Tauri shell: exposes the packing engine and a small file-based catalog to
//! the web UI. All computation runs in-process; there is no server or port.

use omnipack_core::{generate, pack, PackRequest, PackResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
async fn pack_request(request: PackRequest) -> CmdResult<PackResult> {
    tauri::async_runtime::spawn_blocking(move || pack(&request).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn sample_request(kind: String, seed: u64) -> CmdResult<PackRequest> {
    match kind.as_str() {
        "mixed" => Ok(generate::mixed(seed)),
        k if k.starts_with("br") => {
            let class = k[2..].parse().map_err(|_| format!("bad sample `{k}`"))?;
            generate::br_like(class, seed).ok_or_else(|| format!("bad sample `{k}`"))
        }
        _ => Err(format!("unknown sample `{kind}`")),
    }
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

/// Writes a text file chosen by the user through the save dialog.
#[tauri::command]
fn write_text_file(path: String, contents: String) -> CmdResult<()> {
    std::fs::write(&path, contents).map_err(|e| format!("{path}: {e}"))
}

#[tauri::command]
fn read_text_file(path: String) -> CmdResult<String> {
    std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            pack_request,
            sample_request,
            catalog_list,
            catalog_load,
            catalog_save,
            catalog_delete,
            write_text_file,
            read_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OmniPack");
}
