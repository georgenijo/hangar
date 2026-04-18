// End-to-end API test: spawns a real HTTP server with in-memory SQLite and exercises
// a Shell session lifecycle. PTY spawn requires a real terminal allocation, so this
// test is marked #[ignore] and must be run explicitly: `cargo test -- --ignored`.
//
// Run: cargo test -p hangard api_e2e -- --ignored --nocapture

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
#[ignore = "requires PTY allocation; run manually with: cargo test -- --ignored"]
async fn test_shell_session_lifecycle() {
    let (base, _server) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // 1. GET /health
    let resp = client
        .get(format!("{base}/api/v1/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
    assert!(body["uptime_s"].is_number());

    // 2. POST /sessions — create shell session
    let resp = client
        .post(format!("{base}/api/v1/sessions"))
        .json(&serde_json::json!({
            "slug": "test-shell",
            "kind": {"type": "shell"},
            "cols": 80,
            "rows": 24
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let session: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(session["slug"], "test-shell");
    let id = session["id"].as_str().unwrap().to_string();

    // 3. GET /sessions — list
    let resp = client
        .get(format!("{base}/api/v1/sessions"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(list.iter().any(|s| s["id"] == id));

    // 4. GET /sessions/:id — by ULID
    let resp = client
        .get(format!("{base}/api/v1/sessions/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let fetched: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(fetched["id"], id);

    // 5. GET /sessions/test-shell — by slug
    let resp = client
        .get(format!("{base}/api/v1/sessions/test-shell"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let by_slug: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(by_slug["id"], id);

    // 6. PATCH /sessions/:id — update labels
    let resp = client
        .patch(format!("{base}/api/v1/sessions/{id}"))
        .json(&serde_json::json!({"labels": {"env": "test"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let patched: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(patched["labels"]["env"], "test");

    // 7. POST /sessions/:id/prompt
    let resp = client
        .post(format!("{base}/api/v1/sessions/{id}/prompt"))
        .json(&serde_json::json!({"text": "echo hello-hangar"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // 8. Wait for PTY to process
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // 9. GET /sessions/:id/output — check for echo
    let resp = client
        .get(format!("{base}/api/v1/sessions/{id}/output?len=4096"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let output = resp.bytes().await.unwrap();
    assert!(
        output.windows(12).any(|w| w == b"hello-hangar"),
        "expected 'hello-hangar' in PTY output, got: {:?}",
        String::from_utf8_lossy(&output)
    );

    // 10. POST /sessions/:id/key — Enter
    let resp = client
        .post(format!("{base}/api/v1/sessions/{id}/key"))
        .json(&serde_json::json!({"key": "Enter"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // 11. POST /sessions/:id/key — unknown key → 400
    let resp = client
        .post(format!("{base}/api/v1/sessions/{id}/key"))
        .json(&serde_json::json!({"key": "UnknownKey"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // 12. GET /metrics
    let resp = client
        .get(format!("{base}/api/v1/metrics"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let metrics: serde_json::Value = resp.json().await.unwrap();
    assert!(metrics["sessions_active"].is_number());
    assert!(metrics["version"].is_string());

    // 13. POST /broadcast
    let resp = client
        .post(format!("{base}/api/v1/broadcast"))
        .json(&serde_json::json!({"text": "echo broadcast-test"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let br: serde_json::Value = resp.json().await.unwrap();
    assert!(br["sent"].as_u64().unwrap() >= 1);

    // 14. DELETE /sessions/:id
    let resp = client
        .delete(format!("{base}/api/v1/sessions/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // 15. GET /sessions/:id — should return exited state
    let resp = client
        .get(format!("{base}/api/v1/sessions/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let after_delete: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(after_delete["state"], "exited");
}
