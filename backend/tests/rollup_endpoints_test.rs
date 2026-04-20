// Integration tests for rollup endpoints
// Part 1: Tests against live hangard instance (requires manual server start)
// Part 2: Tests with in-memory database and seeded data (standalone)

use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use hangard::{api, db::Db, events::{AgentEvent, Event, EventBus}, session::SessionId, AppState};

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

    // Seed test data: create sample CostUpdated events
    seed_test_data(&db).await;

    let router = api::router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://127.0.0.1:{}", addr.port());

    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (base_url, handle)
}

async fn seed_test_data(db: &Db) {
    // Create a test session
    let session_id = "test-session-1";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    sqlx::query(
        r#"
        INSERT INTO sessions (id, slug, node_id, kind, state, cwd, env, labels, created_at, last_activity_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#
    )
    .bind(session_id)
    .bind("test-session")
    .bind("local")
    .bind(r#"{"type":"shell"}"#)
    .bind(r#""idle""#)
    .bind("/tmp")
    .bind("{}")
    .bind("{}")
    .bind(now)
    .bind(now)
    .execute(db.pool())
    .await
    .unwrap();

    // Insert CostUpdated events for the last few days
    let test_costs = vec![
        (now - 86400000 * 2, 2.5),  // 2 days ago
        (now - 86400000, 1.5),       // 1 day ago
        (now, 3.0),                  // today
    ];

    for (ts, dollars) in test_costs {
        // Create Event::AgentEvent with CostUpdated
        let event = Event::AgentEvent {
            id: session_id.parse::<SessionId>().unwrap(),
            event: AgentEvent::CostUpdated { dollars },
        };

        // Serialize using MessagePack (same as production code)
        let body = rmp_serde::to_vec(&event).unwrap();

        sqlx::query(
            r#"
            INSERT INTO events (session_id, ts, kind, body)
            VALUES (?1, ?2, ?3, ?4)
            "#
        )
        .bind(session_id)
        .bind(ts)
        .bind("AgentEvent")  // Use kind_str() value
        .bind(body)
        .execute(db.pool())
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn test_costs_daily_endpoint() {
    let (base, _server) = spawn_test_server().await;
    let client = reqwest::Client::new();

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
