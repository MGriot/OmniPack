//! `omnipack-server`: the OmniPack API as a standalone service.
//!
//! ```text
//! omnipack-server [--config server.json] [--bind 127.0.0.1:8765] [--api-key KEY]...
//!                 [--insecure-no-auth] [--cors ORIGIN]... [--tls-cert cert.pem --tls-key key.pem]
//!                 [--inbox DIR --outbox DIR [--archive DIR] [--drop-container PRESET|file.json]
//!                  [--drop-options file.json] [--drop-units mm,kg]]
//!                 [--ranker model.json] [--max-jobs N] [--max-sync-seconds S] [--max-job-seconds S] [--quiet]
//! ```
//! Environment: `OMNIPACK_API_KEYS` (comma-separated keys), `OMNIPACK_BIND`.
//! A config file holds the same settings as JSON (docs/api.md).

use omnipack_api::drop::DropConfig;
use omnipack_api::erp::{ErpContainer, Units};
use omnipack_api::{serve, ApiConfig};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: omnipack-server [--config server.json] [--bind 127.0.0.1:8765] [--api-key KEY]... [--insecure-no-auth]
                       [--cors ORIGIN]... [--tls-cert cert.pem --tls-key key.pem]
                       [--inbox DIR --outbox DIR [--archive DIR] [--drop-container PRESET|file.json] [--drop-options file.json] [--drop-units mm,kg]]
                       [--ranker model.json] [--max-jobs N] [--max-sync-seconds S] [--max-job-seconds S] [--quiet]
environment: OMNIPACK_API_KEYS=key1,key2  OMNIPACK_BIND=0.0.0.0:8765
docs: docs/api.md, and GET /api/v1/openapi.json on the running server";

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
}

fn config(args: Vec<String>) -> Result<ApiConfig, String> {
    // The config file first, so flags override it.
    let mut cfg = match args.iter().position(|a| a == "--config") {
        Some(i) => read_json(args.get(i + 1).ok_or("--config needs a file")?)?,
        None => ApiConfig::default(),
    };
    if let Ok(bind) = std::env::var("OMNIPACK_BIND") {
        cfg.bind = bind;
    }
    if let Ok(keys) = std::env::var("OMNIPACK_API_KEYS") {
        cfg.api_keys.extend(keys.split(',').map(str::trim).filter(|k| !k.is_empty()).map(String::from));
    }
    let mut drop = cfg.drop.take();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{a} needs a value"));
        let number = |v: String| v.parse::<f64>().map_err(|_| format!("{a}: not a number"));
        match a.as_str() {
            "--config" => {
                value()?;
            }
            "--bind" => cfg.bind = value()?,
            "--api-key" => cfg.api_keys.push(value()?),
            "--insecure-no-auth" => cfg.insecure_no_auth = true,
            "--cors" => cfg.cors.push(value()?),
            "--tls-cert" => cfg.tls_cert = Some(PathBuf::from(value()?)),
            "--tls-key" => cfg.tls_key = Some(PathBuf::from(value()?)),
            "--ranker" => cfg.ranker_file = Some(PathBuf::from(value()?)),
            "--max-jobs" => cfg.max_jobs = number(value()?)?.max(1.0) as usize,
            "--max-sync-seconds" => cfg.max_sync_seconds = number(value()?)?,
            "--max-job-seconds" => cfg.max_job_seconds = number(value()?)?,
            "--quiet" => cfg.log = false,
            "--inbox" => drop.get_or_insert_with(DropConfig::default).inbox = PathBuf::from(value()?),
            "--outbox" => drop.get_or_insert_with(DropConfig::default).outbox = PathBuf::from(value()?),
            "--archive" => drop.get_or_insert_with(DropConfig::default).archive = Some(PathBuf::from(value()?)),
            "--drop-container" => {
                let v = value()?;
                drop.get_or_insert_with(DropConfig::default).container =
                    if v.ends_with(".json") { read_json(&v)? } else { ErpContainer { preset: Some(v), ..Default::default() } };
            }
            "--drop-options" => drop.get_or_insert_with(DropConfig::default).options = read_json(&value()?)?,
            "--drop-units" => {
                let v = value()?;
                let (length, weight) = v.split_once(',').ok_or("--drop-units: give length,weight, e.g. mm,kg")?;
                drop.get_or_insert_with(DropConfig::default).units = Units { length: length.into(), weight: weight.into() };
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    cfg.drop = drop;
    Ok(cfg)
}

#[tokio::main]
async fn main() -> ExitCode {
    let cfg = match config(std::env::args().skip(1).collect()) {
        Ok(c) => c,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("error: {e}");
            }
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let drop = cfg.drop.as_ref().map(|d| format!("{} → {}", d.inbox.display(), d.outbox.display()));
    let keys = cfg.api_keys.len();
    let handle = match serve(cfg).await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("OmniPack API {} listening on {}", env!("CARGO_PKG_VERSION"), handle.url);
    println!("  spec: {}/api/v1/openapi.json", handle.url);
    println!("  API keys: {}", if keys > 0 { format!("{keys} configured") } else { "none (this computer only)".into() });
    if let Some(d) = drop {
        println!("  drop folders: {d}");
    }
    shutdown_signal().await;
    println!("stopping…");
    handle.stop().await;
    ExitCode::SUCCESS
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("signal handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
