// Integration tests for rollup endpoints
// Spawns a real HTTP server with in-memory SQLite and tests the rollup endpoints
// Run with: cargo test --test rollup_endpoints_test

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use hangard::{api, db::Db, events::EventBus, AppState};

async fn spawn_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let db = Db::new_in_memory().await.unwrap();
    let event_bus = Arc::new(EventBus::new());
    let tmp = tempfile::tempdir().unwrap();
    let ring_dir = tmp.path().to_path_buf();
    // Keep tmpdir alive for the lifetime of the test by leaking it
    std::mem::forget(tmp);

    let logs_config = hangard::config::LogsConfig::default();
    let mut logs_hub = hangard::logs::LogsHub::new(&logs_config, &ring_dir);
    logs_hub.start();

    let state = AppState {
        db,
        event_bus,
        ring_dir,
        hook_channels: Arc::new(Mutex::new(HashMap::new())),
        sessions: Arc::new(RwLock::new(HashMap::new())),
        supervisor: None,
        start_time: Instant::now(),
        sandbox_manager: None,
        logs: Arc::new(logs_hub),
    };

    let router = api::router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://127.0.0.1:{}", addr.port());

    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (base_url, handle)
}

#[tokio::test]
async fn test_pipeline_runs_endpoint() {
    let (base_url, _handle) = spawn_test_server().await;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}/api/v1/pipeline/runs", base_url))
        .send()
        .await
        .expect("request failed");

    // Verify 200 status
    assert_eq!(res.status(), 200, "expected 200 OK status");

    // Verify response is JSON array
    let json: serde_json::Value = res.json().await.expect("invalid json");
    assert!(json.is_array(), "expected JSON array");

    // Verify array contains at least 2 items
    let array = json.as_array().unwrap();
    assert!(
        array.len() >= 2,
        "expected at least 2 pipeline runs, got {}",
        array.len()
    );

    // Verify first object has all required fields
    let first = &array[0];
    assert!(first.get("issue").is_some(), "missing 'issue' field");
    assert!(first.get("title").is_some(), "missing 'title' field");
    assert!(first.get("state").is_some(), "missing 'state' field");
    assert!(first.get("phase").is_some(), "missing 'phase' field");
    assert!(first.get("cost").is_some(), "missing 'cost' field");
    assert!(first.get("tokens").is_some(), "missing 'tokens' field");
    assert!(first.get("agents").is_some(), "missing 'agents' field");
    assert!(
        first.get("duration_s").is_some(),
        "missing 'duration_s' field"
    );

    // Verify second object has all required fields
    let second = &array[1];
    assert!(second.get("issue").is_some(), "missing 'issue' field");
    assert!(second.get("title").is_some(), "missing 'title' field");
    assert!(second.get("state").is_some(), "missing 'state' field");
    assert!(second.get("phase").is_some(), "missing 'phase' field");
    assert!(second.get("cost").is_some(), "missing 'cost' field");
    assert!(second.get("tokens").is_some(), "missing 'tokens' field");
    assert!(second.get("agents").is_some(), "missing 'agents' field");
    assert!(
        second.get("duration_s").is_some(),
        "missing 'duration_s' field"
    );
}
