// Integration tests for rollup endpoints
// Part 1: Tests against live hangard instance (requires manual server start)
// Part 2: Tests with in-memory database and seeded data (standalone)

use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use hangard::{api, db::Db, events::{AgentEvent, Event, EventBus, EventStore}, session::{Session, SessionId, SessionKind, SessionState}, AppState};

// ===== Part 1: Manual tests against live server =====

// NOTE: These tests require a running hangard instance on localhost:3000
// Run with: cargo test --test rollup_endpoints_test -- --ignored

#[tokio::test]
#[ignore]
async fn test_host_metrics_endpoint() {
    let client = Client::new();
    let res = client
        .get("http://localhost:3000/api/v1/metrics/host")
        .send()
        .await
        .expect("request failed");

    assert_eq!(res.status(), 200);

    let json: Value = res.json().await.expect("invalid json");
    assert!(json.get("hostname").is_some(), "missing hostname");
    assert!(json.get("cpu_pct").is_some(), "missing cpu_pct");
    assert!(json.get("ram_used_bytes").is_some(), "missing ram_used_bytes");
    assert!(json.get("ram_total_bytes").is_some(), "missing ram_total_bytes");
    assert!(json.get("disk_used_bytes").is_some(), "missing disk_used_bytes");
    assert!(json.get("disk_total_bytes").is_some(), "missing disk_total_bytes");
}

#[tokio::test]
#[ignore]
async fn test_host_metrics_cache() {
    let client = Client::new();

    // First call - should hit the actual system
    let start1 = std::time::Instant::now();
    let res1 = client
        .get("http://localhost:3000/api/v1/metrics/host")
        .send()
        .await
        .expect("request failed");
    let duration1 = start1.elapsed();

    assert_eq!(res1.status(), 200);
    let json1: Value = res1.json().await.expect("invalid json");

    // Second call immediately after - should hit cache and be faster
    let start2 = std::time::Instant::now();
    let res2 = client
        .get("http://localhost:3000/api/v1/metrics/host")
        .send()
        .await
        .expect("request failed");
    let duration2 = start2.elapsed();

    assert_eq!(res2.status(), 200);
    let json2: Value = res2.json().await.expect("invalid json");

    // Cached response should be identical
    assert_eq!(json1, json2, "cached response should match");

    // Note: Response time comparison is not reliable in tests due to network variance
    // but we can at least verify the cache returns valid data
    println!("First request: {:?}, Second request: {:?}", duration1, duration2);
}

// ===== Part 2: Standalone tests with in-memory DB =====

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
    let event_bus = Arc::new(EventBus::new());
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
async fn test_costs_daily_endpoint() {
    let (base, _server, db) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Seed test data with CostUpdated events
    let session_id = SessionId::from_str("test-session-1").unwrap();
    make_session(&session_id).insert(db.pool()).await.unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Insert CostUpdated events for the last few days
    let test_costs = vec![
        (now - 86400000 * 2, 2.5),  // 2 days ago
        (now - 86400000, 1.5),       // 1 day ago
        (now, 3.0),                  // today
    ];

    for (ts, dollars) in test_costs {
        EventStore::insert(
            db.pool(),
            session_id.as_ref(),
            &Event::AgentEvent {
                id: session_id.clone(),
                event: AgentEvent::CostUpdated { dollars },
            },
        )
        .await
        .unwrap();

        // Update ts manually since EventStore uses current timestamp
        sqlx::query("UPDATE events SET ts = ? WHERE ts = (SELECT MAX(ts) FROM events)")
            .bind(ts)
            .execute(db.pool())
            .await
            .unwrap();
    }

    let resp = client
        .get(format!("{base}/api/v1/costs/daily"))
        .send()
        .await
        .expect("request failed");

    // AC1: Returns 200 status
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.expect("invalid json");

    // AC2: Returns JSON array
    assert!(json.is_array(), "expected array response");

    let arr = json.as_array().unwrap();

    // AC4: Verify array length >= 1 and elements contain date and dollars fields
    assert!(arr.len() >= 1, "expected at least one element in array");

    for item in arr {
        // AC3: Array elements contain date field
        assert!(item.get("date").is_some(), "missing date field");
        let date_str = item.get("date").unwrap().as_str().unwrap();

        // AC3: Verify date format is YYYY-MM-DD
        assert!(
            date_str.len() == 10 && date_str.chars().filter(|c| *c == '-').count() == 2,
            "date should be in YYYY-MM-DD format, got: {}",
            date_str
        );

        // AC3: Array elements contain dollars field
        assert!(item.get("dollars").is_some(), "missing dollars field");
        assert!(
            item.get("dollars").unwrap().is_number(),
            "dollars should be a number"
        );
    }

    // Additional validation: verify we can parse the response as Vec<DailyCost>
    let first = &arr[0];
    assert!(first["date"].is_string());
    assert!(first["dollars"].is_f64() || first["dollars"].is_i64());
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

    // Session 3 events (no model set)
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

    // Make request
    let resp = client
        .get(format!("{base}/api/v1/costs/by-model"))
        .send()
        .await
        .unwrap();

    // AC1: Returns 200 status
    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();

    // AC2: Returns JSON array
    assert!(json.is_array(), "expected array response");

    let array = json.as_array().unwrap();

    // AC3: Should have 3 entries (opus, sonnet, unknown)
    assert_eq!(array.len(), 3);

    // AC4: Verify sorted by dollars descending
    // First should be opus ($3.50)
    assert_eq!(array[0]["model"], "claude-opus-4");
    assert_eq!(array[0]["dollars"], 3.50);

    // Second should be unknown ($1.20)
    assert_eq!(array[1]["model"], "unknown");
    assert_eq!(array[1]["dollars"], 1.20);

    // Third should be sonnet ($0.80)
    assert_eq!(array[2]["model"], "claude-sonnet-4");
    assert_eq!(array[2]["dollars"], 0.80);
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
