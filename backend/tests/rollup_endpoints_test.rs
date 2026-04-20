// Integration tests for rollup endpoints in backend/src/api/rollup.rs
//
// Run: cargo test --test rollup_endpoints_test

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use hangard::{api, db::Db, events::{AgentEvent, Event, EventStore}, AppState};
use hangard::session::{Session, SessionId, SessionKind, SessionState};

fn make_session(id: &SessionId) -> Session {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    Session {
        id: id.clone(),
        slug: format!("test-{}", id),
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

async fn spawn_test_server() -> (String, tokio::task::JoinHandle<()>, Db) {
    let db = Db::new_in_memory().await.unwrap();
    let event_bus = Arc::new(hangard::events::EventBus::new());
    let tmp = tempfile::tempdir().unwrap();
    let ring_dir = tmp.path().to_path_buf();
    // Keep tmpdir alive for the lifetime of the test by leaking it
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

    (base_url, handle, db)
}

#[tokio::test]
async fn test_costs_by_model_endpoint() {
    let (base, _server, db) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Seed test database with diverse data:
    // - Session 1: model "claude-opus-4", costs: $2.50 + $1.00 = $3.50
    // - Session 2: model "claude-sonnet-4", costs: $0.50 + $0.30 = $0.80
    // - Session 3: no model (should default to 'unknown'), costs: $1.20

    let session1 = SessionId::from_str("session-1").unwrap();
    let session2 = SessionId::from_str("session-2").unwrap();
    let session3 = SessionId::from_str("session-3").unwrap();

    // Create sessions in database
    make_session(&session1).insert(db.pool()).await.unwrap();
    make_session(&session2).insert(db.pool()).await.unwrap();
    make_session(&session3).insert(db.pool()).await.unwrap();

    // Session 1 events
    EventStore::insert(
        db.pool(),
        session1.as_ref(),
        &Event::AgentEvent {
            id: session1.clone(),
            event: AgentEvent::ModelChanged {
                model: "claude-opus-4".to_string(),
            },
        },
    )
    .await
    .unwrap();

    EventStore::insert(
        db.pool(),
        session1.as_ref(),
        &Event::AgentEvent {
            id: session1.clone(),
            event: AgentEvent::CostUpdated { dollars: 2.50 },
        },
    )
    .await
    .unwrap();

    EventStore::insert(
        db.pool(),
        session1.as_ref(),
        &Event::AgentEvent {
            id: session1.clone(),
            event: AgentEvent::CostUpdated { dollars: 1.00 },
        },
    )
    .await
    .unwrap();

    // Session 2 events
    EventStore::insert(
        db.pool(),
        session2.as_ref(),
        &Event::AgentEvent {
            id: session2.clone(),
            event: AgentEvent::ModelChanged {
                model: "claude-sonnet-4".to_string(),
            },
        },
    )
    .await
    .unwrap();

    EventStore::insert(
        db.pool(),
        session2.as_ref(),
        &Event::AgentEvent {
            id: session2.clone(),
            event: AgentEvent::CostUpdated { dollars: 0.50 },
        },
    )
    .await
    .unwrap();

    EventStore::insert(
        db.pool(),
        session2.as_ref(),
        &Event::AgentEvent {
            id: session2.clone(),
            event: AgentEvent::CostUpdated { dollars: 0.30 },
        },
    )
    .await
    .unwrap();

    // Session 3 events (no model - should default to 'unknown')
    EventStore::insert(
        db.pool(),
        session3.as_ref(),
        &Event::AgentEvent {
            id: session3.clone(),
            event: AgentEvent::CostUpdated { dollars: 1.20 },
        },
    )
    .await
    .unwrap();

    // Make HTTP request to /api/v1/costs/by-model
    let resp = client
        .get(format!("{base}/api/v1/costs/by-model"))
        .send()
        .await
        .unwrap();

    // Verify 200 status
    assert_eq!(resp.status(), 200);

    // Parse JSON response
    let json: serde_json::Value = resp.json().await.unwrap();

    // Verify it's an array
    assert!(json.is_array(), "Response should be an array");
    let array = json.as_array().unwrap();

    // Verify array has 3 elements (3 models: opus, sonnet, unknown)
    assert_eq!(array.len(), 3, "Expected 3 model cost entries");

    // Verify structure: each element has 'model' and 'dollars' fields
    for item in array {
        assert!(item.get("model").is_some(), "Missing 'model' field");
        assert!(item.get("dollars").is_some(), "Missing 'dollars' field");
    }

    // Verify sorting: results should be sorted by dollars descending
    // Expected order: claude-opus-4 ($3.50), unknown ($1.20), claude-sonnet-4 ($0.80)
    assert_eq!(array[0]["model"], "claude-opus-4");
    assert_eq!(array[0]["dollars"], 3.50);

    assert_eq!(array[1]["model"], "unknown");
    assert_eq!(array[1]["dollars"], 1.20);

    assert_eq!(array[2]["model"], "claude-sonnet-4");
    assert_eq!(array[2]["dollars"], 0.80);

    // Verify AC4: jq expression compatibility
    // length >= 1 and .[0].model and .[0].dollars
    assert!(array.len() >= 1);
    assert!(array[0].get("model").is_some());
    assert!(array[0].get("dollars").is_some());
}

#[tokio::test]
async fn test_costs_by_model_empty_database() {
    let (base, _server, _db) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Make request to empty database
    let resp = client
        .get(format!("{base}/api/v1/costs/by-model"))
        .send()
        .await
        .unwrap();

    // Should return 200 with empty array
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_costs_by_model_model_change_updates() {
    let (base, _server, db) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Test case: session changes model mid-session
    // - Session 1: starts with opus, changes to sonnet, costs should be attributed to sonnet (most recent)

    let session1 = SessionId::from_str("session-change").unwrap();

    // Create session in database
    make_session(&session1).insert(db.pool()).await.unwrap();

    // Initial model: opus
    EventStore::insert(
        db.pool(),
        session1.as_ref(),
        &Event::AgentEvent {
            id: session1.clone(),
            event: AgentEvent::ModelChanged {
                model: "claude-opus-4".to_string(),
            },
        },
    )
    .await
    .unwrap();

    EventStore::insert(
        db.pool(),
        session1.as_ref(),
        &Event::AgentEvent {
            id: session1.clone(),
            event: AgentEvent::CostUpdated { dollars: 1.00 },
        },
    )
    .await
    .unwrap();

    // Model change to sonnet
    EventStore::insert(
        db.pool(),
        session1.as_ref(),
        &Event::AgentEvent {
            id: session1.clone(),
            event: AgentEvent::ModelChanged {
                model: "claude-sonnet-4".to_string(),
            },
        },
    )
    .await
    .unwrap();

    EventStore::insert(
        db.pool(),
        session1.as_ref(),
        &Event::AgentEvent {
            id: session1.clone(),
            event: AgentEvent::CostUpdated { dollars: 2.00 },
        },
    )
    .await
    .unwrap();

    // Make request
    let resp = client
        .get(format!("{base}/api/v1/costs/by-model"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();
    let array = json.as_array().unwrap();

    // Should have only sonnet with total $3.00 (all costs attributed to last model)
    assert_eq!(array.len(), 1);
    assert_eq!(array[0]["model"], "claude-sonnet-4");
    assert_eq!(array[0]["dollars"], 3.00);
}
