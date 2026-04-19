// Regression tests for #59: concurrent DELETE /sessions/:id must return one
// 204 and one 404, not two 204s.
//
// No PTY is needed — sessions are inserted directly into the DB.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use hangard::{
    api,
    db::Db,
    events::EventBus,
    session::{Session, SessionId, SessionKind, SessionState},
    AppState,
};

async fn spawn_test_server() -> (String, Db, tokio::task::JoinHandle<()>) {
    let db = Db::new_in_memory().await.unwrap();
    let event_bus = Arc::new(EventBus::new());
    let tmp = tempfile::tempdir().unwrap();
    let ring_dir = tmp.path().to_path_buf();
    std::mem::forget(tmp);

    let logs_config = hangard::config::LogsConfig::default();
    let mut logs_hub = hangard::logs::LogsHub::new(&logs_config, &ring_dir);
    logs_hub.start();

    let state = AppState {
        db: db.clone(),
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

    (base_url, db, handle)
}

fn make_session() -> Session {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    Session {
        id: SessionId::new(),
        slug: format!("race-test-{}", ulid::Ulid::new()),
        node_id: "local".to_string(),
        kind: SessionKind::Shell,
        state: SessionState::Idle,
        cwd: "/tmp".to_string(),
        env: serde_json::json!({}),
        agent_meta: None,
        labels: serde_json::json!({}),
        created_at: now,
        last_activity_at: now,
        exit: None,
        sandbox: None,
    }
}

#[tokio::test]
async fn concurrent_delete_returns_one_204_one_404() {
    let (base, db, _server) = spawn_test_server().await;

    // Insert session directly — no PTY required.
    let session = make_session();
    let id = session.id.to_string();
    session.insert(db.pool()).await.unwrap();

    let client = reqwest::Client::new();
    let url = format!("{base}/api/v1/sessions/{id}");

    let (r1, r2) = tokio::join!(client.delete(&url).send(), client.delete(&url).send(),);

    let mut statuses = vec![r1.unwrap().status().as_u16(), r2.unwrap().status().as_u16()];
    statuses.sort_unstable();
    assert_eq!(statuses, vec![204, 404]);
}

#[tokio::test]
async fn delete_nonexistent_returns_404() {
    let (base, _db, _server) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let fake_id = ulid::Ulid::new().to_string();
    let resp = client
        .delete(format!("{base}/api/v1/sessions/{fake_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404);
}
