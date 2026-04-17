use axum::{extract::State, http::StatusCode, Json};

use crate::AppState;

#[derive(sqlx::FromRow)]
struct StateCount {
    state: String,
    count: i64,
}

#[derive(sqlx::FromRow)]
struct MetricsDaily {
    tokens_total: i64,
    push_sent: i64,
}

pub async fn get_metrics(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sessions_active = state.sessions.read().unwrap().len();

    let state_counts = sqlx::query_as::<_, StateCount>(
        "SELECT state, COUNT(*) as count FROM sessions GROUP BY state",
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut sessions_by_state = serde_json::Map::new();
    for row in state_counts {
        let key = row.state.trim_matches('"').to_string();
        sessions_by_state.insert(key, serde_json::Value::Number(row.count.into()));
    }

    let daily = sqlx::query_as::<_, MetricsDaily>(
        "SELECT tokens_total, push_sent FROM metrics_daily WHERE day = strftime('%Y-%m-%d', 'now')",
    )
    .fetch_optional(state.db.pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (tokens_today, push_sent_today) = daily
        .map(|r| (r.tokens_total, r.push_sent))
        .unwrap_or((0, 0));

    let rss_mb = read_rss_mb();
    let uptime_s = state.start_time.elapsed().as_secs();

    Ok(Json(serde_json::json!({
        "sessions_active": sessions_active,
        "sessions_by_state": sessions_by_state,
        "tokens_today": tokens_today,
        "push_sent_today": push_sent_today,
        "rss_mb": rss_mb,
        "uptime_s": uptime_s,
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

#[cfg(target_os = "linux")]
fn read_rss_mb() -> f64 {
    use std::io::{BufRead, BufReader};
    let file = match std::fs::File::open("/proc/self/status") {
        Ok(f) => f,
        Err(_) => return 0.0,
    };
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if line.starts_with("VmRSS:") {
            let kb: u64 = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            return (kb as f64) / 1024.0;
        }
    }
    0.0
}

#[cfg(not(target_os = "linux"))]
fn read_rss_mb() -> f64 {
    0.0
}
