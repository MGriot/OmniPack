//! The API end to end, in process: auth, both formats and their units, CSV,
//! validation, jobs with callbacks, drop folders and the OpenAPI examples.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use omnipack_api::drop::{scan_once, DropConfig};
use omnipack_api::engine::Engine;
use omnipack_api::erp::ErpRequest;
use omnipack_api::jobs::JobSpec;
use omnipack_api::{router, ApiConfig, AppState, OPENAPI};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower::ServiceExt;

fn app() -> axum::Router {
    router(AppState::new(ApiConfig { api_keys: vec!["k".into()], log: false, ..Default::default() }))
}

async fn call(app: &axum::Router, method: &str, uri: &str, body: Option<Value>, key: Option<&str>) -> (StatusCode, String, Value) {
    let mut b = Request::builder().method(method).uri(uri).header("content-type", "application/json");
    if let Some(k) = key {
        b = b.header("x-api-key", k);
    }
    let req = b.body(body.map_or(Body::empty(), |v| Body::from(v.to_string()))).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let ctype = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let v = serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (status, ctype, v)
}

fn example(name: &str) -> Value {
    let spec: Value = serde_json::from_str(OPENAPI).unwrap();
    spec["components"]["examples"][name]["value"].clone()
}

#[tokio::test]
async fn keys_are_required_except_for_health() {
    let a = app();
    assert_eq!(call(&a, "GET", "/api/v1/health", None, None).await.0, StatusCode::OK);
    assert_eq!(call(&a, "GET", "/api/v1/openapi.json", None, None).await.0, StatusCode::OK);
    assert_eq!(call(&a, "GET", "/api/v1/presets", None, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&a, "GET", "/api/v1/presets", None, Some("wrong")).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&a, "GET", "/api/v1/presets", None, Some("k")).await.0, StatusCode::OK);
    let req = Request::get("/api/v1/presets").header("authorization", "Bearer k").body(Body::empty()).unwrap();
    assert_eq!(a.clone().oneshot(req).await.unwrap().status(), StatusCode::OK);
    assert_eq!(call(&a, "GET", "/api/v1/nothing", None, Some("k")).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn erp_plan_answers_in_the_request_units() {
    let a = app();
    let req = example("erpDelivery");
    let (status, _, r) = call(&a, "POST", "/api/v1/erp/plan", Some(req.clone()), Some("k")).await;
    assert_eq!(status, StatusCode::OK, "{r}");
    assert_eq!(r["reference"], "80001234");
    assert_eq!(r["summary"]["units_placed"], 30);
    assert_eq!(r["status"], "ok", "{}", r["containers"][0]["violations"]);
    // The same request through the engine directly, in mm.
    let erp: ErpRequest = serde_json::from_value(req).unwrap();
    let plan = erp.to_plan(60.0, None).unwrap();
    assert_eq!(plan.mm, 10.0);
    let direct = omnipack_core::pack(&plan.request).unwrap();
    let first = &direct.containers[0].placements[0];
    let p = &r["placements"][0];
    assert_eq!(p["item_id"], first.item_id.as_str());
    for (k, axis) in [("x", 0), ("y", 1), ("z", 2)] {
        assert!((p[k].as_f64().unwrap() - first.position[axis] / 10.0).abs() < 1e-3, "{k}: {p}");
    }
    assert!((p["length"].as_f64().unwrap() - first.size[2] / 10.0).abs() < 1e-3);
    // A 40 ft high cube from the preset: 1203.2 cm long.
    assert!((r["containers"][0]["length"].as_f64().unwrap() - 1203.2).abs() < 1e-6);
}

#[tokio::test]
async fn weights_come_back_in_pounds() {
    let a = app();
    let req = json!({
        "units": { "length": "INH", "weight": "LBR" },
        "container": { "length": 236, "width": 92, "height": 94 },
        "items": [{ "id": "crate", "quantity": 3, "length": 40, "width": 30, "height": 20, "weight": 110.0 }]
    });
    let (status, _, r) = call(&a, "POST", "/api/v1/erp/plan", Some(req), Some("k")).await;
    assert_eq!(status, StatusCode::OK, "{r}");
    assert!((r["summary"]["total_weight"].as_f64().unwrap() - 330.0).abs() < 1e-6, "{}", r["summary"]);
    assert_eq!(r["units"]["weight"], "LBR");
    let bad = json!({ "units": { "length": "furlong" }, "container": { "preset": "20ft-dv" }, "items": [] });
    let (status, _, e) = call(&a, "POST", "/api/v1/erp/plan", Some(bad), Some("k")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(e["error"].as_str().unwrap().contains("furlong"));
}

#[tokio::test]
async fn csv_load_list_on_request() {
    let a = app();
    let (status, ctype, body) = call(&a, "POST", "/api/v1/erp/plan?format=csv", Some(example("erpDelivery")), Some("k")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(ctype.starts_with("text/csv"), "{ctype}");
    let text = body.as_str().unwrap();
    assert!(text.starts_with("container,seq,unit,item"));
    assert_eq!(text.lines().count(), 31, "header + 30 units");
}

#[tokio::test]
async fn full_format_pack() {
    let a = app();
    let req = serde_json::to_value(omnipack_core::generate::mixed(1)).unwrap();
    let (status, _, r) = call(&a, "POST", "/api/v1/pack", Some(req), Some("k")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(r["schema"], "omnipack.plan/1");
    assert!(r["containers"][0]["violations"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn validation_flags_what_is_wrong() {
    let a = app();
    let (status, _, r) = call(&a, "POST", "/api/v1/validate", Some(example("erpValidate")), Some("k")).await;
    assert_eq!(status, StatusCode::OK, "{r}");
    assert_eq!(r["status"], "ok", "{}", r["containers"][0]["violations"]);
    // The second pallet inside the first one.
    let mut bad = example("erpValidate");
    bad["placements"][1]["y"] = json!(500);
    let (_, _, r) = call(&a, "POST", "/api/v1/validate", Some(bad), Some("k")).await;
    assert_eq!(r["status"], "invalid");
    assert!(r["containers"][0]["violations"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().starts_with("overlap")), "{r}");
}

async fn wait_done(a: &axum::Router, id: &str) -> Value {
    for _ in 0..300 {
        let (_, _, j) = call(a, "GET", &format!("/api/v1/jobs/{id}"), None, Some("k")).await;
        if ["done", "failed", "cancelled"].contains(&j["status"].as_str().unwrap_or("")) {
            return j;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("job {id} did not finish");
}

#[tokio::test(flavor = "multi_thread")]
async fn jobs_run_and_call_back() {
    // A receiver standing in for the caller's endpoint.
    // (Authorization header, body) of each call received.
    type Received = Arc<Mutex<Vec<(Option<String>, Value)>>>;
    let got: Received = Arc::default();
    let sink = got.clone();
    let receiver = axum::Router::new().route(
        "/hook",
        axum::routing::post(move |headers: axum::http::HeaderMap, axum::Json(v): axum::Json<Value>| {
            let sink = sink.clone();
            async move {
                let auth = headers.get("authorization").and_then(|h| h.to_str().ok()).map(String::from);
                sink.lock().unwrap().push((auth, v));
                "ok"
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let hook = format!("http://{}/hook", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, receiver).await.unwrap() });

    let a = app();
    let mut spec = example("jobSearch");
    spec["budget_s"] = json!(1);
    spec["callback"] = json!({ "url": hook, "headers": { "Authorization": "Basic dGVzdA==" } });
    let (status, _, j) = call(&a, "POST", "/api/v1/jobs", Some(spec), Some("k")).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{j}");
    let done = wait_done(&a, j["id"].as_str().unwrap()).await;
    assert_eq!(done["status"], "done", "{done}");
    assert_eq!(done["result"]["summary"]["units_placed"], 40);
    for _ in 0..100 {
        if !got.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let calls = got.lock().unwrap().clone();
    assert_eq!(calls.len(), 1, "one callback");
    assert_eq!(calls[0].0.as_deref(), Some("Basic dGVzdA=="));
    assert_eq!(calls[0].1["reference"], "80001234");
    assert_eq!(calls[0].1["status"], "done");
    let (_, _, j) = call(&a, "GET", &format!("/api/v1/jobs/{}", done["id"].as_str().unwrap()), None, Some("k")).await;
    assert_eq!(j["callback"]["delivered"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cancelled_search_keeps_its_best_plan() {
    let a = app();
    let spec = json!({ "kind": "best", "budget_s": 30, "erp": example("erpDelivery") });
    let (_, _, j) = call(&a, "POST", "/api/v1/jobs", Some(spec), Some("k")).await;
    let id = j["id"].as_str().unwrap().to_string();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let (status, _, _) = call(&a, "DELETE", &format!("/api/v1/jobs/{id}"), None, Some("k")).await;
    assert_eq!(status, StatusCode::OK);
    let done = wait_done(&a, &id).await;
    assert_eq!(done["status"], "cancelled", "{}", done["status"]);
    assert_eq!(done["result"]["summary"]["units_placed"], 30, "the best plan so far comes back");
    let (_, _, missing) = call(&a, "GET", "/api/v1/jobs/nope", None, Some("k")).await;
    assert!(missing["error"].as_str().unwrap().contains("nope"));
}

#[test]
fn drop_folders_process_csv_and_report_errors() {
    let dir = std::env::temp_dir().join(format!("omnipack-drop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = DropConfig { inbox: dir.join("in"), outbox: dir.join("out"), ..Default::default() };
    std::fs::create_dir_all(&cfg.inbox).unwrap();
    // SAP-style export: semicolons, decimal commas, X for true.
    std::fs::write(cfg.inbox.join("80001234.csv"), "MATNR;Description;Quantity;Length;Width;Height;Weight;This side up\nMAT-1;Pallet;4;1200;800;1000;450,5;X\nMAT-2;Carton;10;400;300;300;12;\n").unwrap();
    std::fs::write(cfg.inbox.join("broken.json"), "{ not json").unwrap();
    let done = scan_once(&cfg, &Engine::default(), Duration::ZERO);
    assert_eq!(done.len(), 2);
    let result: Value = serde_json::from_str(&std::fs::read_to_string(cfg.outbox.join("80001234.result.json")).unwrap()).unwrap();
    assert_eq!(result["reference"], "80001234");
    assert_eq!(result["summary"]["units_placed"], 14);
    assert!((result["summary"]["total_weight"].as_f64().unwrap() - (4.0 * 450.5 + 120.0)).abs() < 1e-6);
    assert!(std::fs::read_to_string(cfg.outbox.join("80001234.loadlist.csv")).unwrap().lines().count() == 15);
    assert!(std::fs::read_to_string(cfg.outbox.join("broken.error.txt")).unwrap().contains("broken.json"));
    assert!(cfg.inbox.join("archive").join("80001234.csv").exists());
    assert!(!cfg.inbox.join("80001234.csv").exists(), "never processed twice");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sap_field_names_work_as_csv_columns() {
    let items = omnipack_api::csv::parse_items("MATNR;MAKTX;LFIMG;LAENG;BREIT;HOEHE;BRGEW;STACKABLE
M1;Pallet A;4;1200;800;1000;300,5;X
M2;Crate;6;600;400;400;45;
").unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!((items[0].id.as_str(), items[0].description.as_deref(), items[0].quantity), ("M1", Some("Pallet A"), 4));
    assert_eq!((items[0].length, items[0].width, items[0].height, items[0].weight), (1200.0, 800.0, 1000.0, 300.5));
    assert!(items[0].stackable && !items[1].stackable);
}

#[test]
fn openapi_examples_are_real_requests() {
    let spec: Value = serde_json::from_str(OPENAPI).unwrap();
    assert_eq!(spec["openapi"], "3.1.0");
    for name in ["erpDelivery", "erpValidate"] {
        let r: Result<ErpRequest, _> = serde_json::from_value(example(name));
        assert!(r.is_ok(), "{name}: {:?}", r.err());
        assert!(r.unwrap().to_plan(60.0, None).is_ok(), "{name}");
    }
    let j: Result<JobSpec, _> = serde_json::from_value(example("jobSearch"));
    assert!(j.is_ok(), "{:?}", j.err());
    // Every documented path is routed.
    for path in spec["paths"].as_object().unwrap().keys() {
        assert!(path.starts_with("/api/v1/"), "{path}");
    }
}
