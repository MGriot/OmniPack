//! The local API: the same HTTP API as `omnipack-server`, served by the
//! desktop app while it runs (not on phones). Settings live in the app data
//! folder as `api.json`.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalApiSettings {
    pub enabled: bool,
    pub port: u16,
    /// Required by callers when non-empty (and always when other computers may connect).
    pub api_key: String,
    /// Listen on all network interfaces instead of this computer only.
    pub allow_network: bool,
    /// Drop folders (both or neither).
    pub inbox: Option<String>,
    pub outbox: Option<String>,
}

impl Default for LocalApiSettings {
    fn default() -> Self {
        LocalApiSettings { enabled: false, port: 8765, api_key: String::new(), allow_network: false, inbox: None, outbox: None }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiStatus {
    /// False on phones: the local API is a desktop feature.
    pub supported: bool,
    pub running: bool,
    pub url: Option<String>,
    pub error: Option<String>,
    pub settings: LocalApiSettings,
}

fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("api.json"))
}

pub fn read_settings(app: &AppHandle) -> LocalApiSettings {
    settings_path(app).ok().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

#[cfg(desktop)]
fn write_settings(app: &AppHandle, s: &LocalApiSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

#[cfg(desktop)]
pub mod imp {
    use super::*;
    use omnipack_api::drop::DropConfig;
    use omnipack_api::{serve, ApiConfig, ServeHandle};
    use tauri::async_runtime::Mutex;
    use tauri::State;

    /// The running server, if any, and the last start error.
    #[derive(Default)]
    pub struct LocalApi {
        handle: Mutex<Option<ServeHandle>>,
        error: std::sync::Mutex<Option<String>>,
    }

    fn config(app: &AppHandle, s: &LocalApiSettings) -> Result<ApiConfig, String> {
        if s.allow_network && s.api_key.trim().is_empty() {
            return Err("Set an API key before allowing other computers to connect.".into());
        }
        let mut cfg = ApiConfig {
            bind: format!("{}:{}", if s.allow_network { "0.0.0.0" } else { "127.0.0.1" }, s.port),
            api_keys: Some(s.api_key.trim().to_string()).filter(|k| !k.is_empty()).into_iter().collect(),
            log: false,
            ..Default::default()
        };
        // The app's learned model, if trained.
        cfg.ranker_file = app.path().app_data_dir().ok().map(|d| d.join("model.json")).filter(|p| p.exists());
        if let (Some(i), Some(o)) = (&s.inbox, &s.outbox) {
            if !i.is_empty() && !o.is_empty() {
                cfg.drop = Some(DropConfig { inbox: i.into(), outbox: o.into(), ..Default::default() });
            }
        }
        Ok(cfg)
    }

    async fn restart(app: &AppHandle, api: &LocalApi, s: &LocalApiSettings) {
        let mut slot = api.handle.lock().await;
        if let Some(h) = slot.take() {
            h.stop().await;
        }
        let outcome = match (s.enabled, config(app, s)) {
            (false, _) => Ok(None),
            (true, Err(e)) => Err(e),
            (true, Ok(cfg)) => serve(cfg).await.map(Some),
        };
        let mut err = api.error.lock().unwrap();
        match outcome {
            Ok(h) => {
                *slot = h;
                *err = None;
            }
            Err(e) => *err = Some(e),
        }
    }

    async fn status(app: &AppHandle, api: &LocalApi) -> ApiStatus {
        let slot = api.handle.lock().await;
        let running = slot.as_ref().is_some_and(|h| h.is_running());
        ApiStatus {
            supported: true,
            running,
            url: slot.as_ref().filter(|_| running).map(|h| h.url.clone()),
            error: api.error.lock().unwrap().clone(),
            settings: read_settings(app),
        }
    }

    /// Starts the API with the app when it is enabled.
    pub fn autostart(app: &AppHandle) {
        let s = read_settings(app);
        if s.enabled {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let api = app.state::<LocalApi>();
                restart(&app, &api, &s).await;
            });
        }
    }

    /// After retraining, the running API picks up the new model.
    pub async fn reload(app: &AppHandle) {
        let s = read_settings(app);
        if s.enabled {
            let api = app.state::<LocalApi>();
            restart(app, &api, &s).await;
        }
    }

    #[tauri::command]
    pub async fn api_status(app: AppHandle, api: State<'_, LocalApi>) -> Result<ApiStatus, String> {
        Ok(status(&app, &api).await)
    }

    /// Saves the settings and starts, restarts or stops the API to match.
    #[tauri::command]
    pub async fn api_apply(app: AppHandle, api: State<'_, LocalApi>, settings: LocalApiSettings) -> Result<ApiStatus, String> {
        write_settings(&app, &settings)?;
        restart(&app, &api, &settings).await;
        Ok(status(&app, &api).await)
    }
}

#[cfg(mobile)]
pub mod imp {
    use super::*;

    #[derive(Default)]
    pub struct LocalApi;

    pub fn autostart(_: &AppHandle) {}

    pub async fn reload(_: &AppHandle) {}

    #[tauri::command]
    pub async fn api_status(app: AppHandle) -> Result<ApiStatus, String> {
        Ok(ApiStatus { supported: false, running: false, url: None, error: None, settings: read_settings(&app) })
    }

    #[tauri::command]
    pub async fn api_apply(_settings: LocalApiSettings) -> Result<ApiStatus, String> {
        Err("The local API runs in the desktop app only; use omnipack-server for other systems.".into())
    }
}

// The commands are registered as `local_api::imp::…` (Tauri's command macros
// are not visible through a re-export).
pub use imp::{autostart, reload, LocalApi};
