use reqwest::Client;
use serde_json::Value;

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
