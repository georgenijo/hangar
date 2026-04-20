use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use sysinfo::{Disks, System};

use crate::events::{AgentEvent, Event};

// ===== Types =====

#[derive(Debug, Clone, Serialize)]
pub struct HostMetrics {
    pub hostname: String,
    pub cpu_pct: f32,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyCost {
    pub date: String,
    pub dollars: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelCost {
    pub model: String,
    pub dollars: f64,
}

/// State enum with serde rename to match frontend string literals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PipelineRunState {
    Pending,
    Live,
    Done,
    Failed,
}

/// Phase enum with serde rename to match frontend string literals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PipelinePhaseId {
    Planner,
    Architect,
    Reviewer,
    Builder,
    Tester,
    Fixer,
    Pr,
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineRun {
    pub issue: u32,
    pub title: String,
    pub state: PipelineRunState,
    pub phase: PipelinePhaseId,
    pub cost: f64,
    pub tokens: u64,
    pub agents: u32,
    pub duration_s: u64,
}

// ===== Metrics Cache (2-second TTL) =====

struct MetricsCache {
    metrics: Option<HostMetrics>,
    updated_at: Instant,
}

static METRICS_CACHE: Mutex<Option<MetricsCache>> = Mutex::new(None);

// ===== Handler: GET /api/v1/metrics/host =====

pub async fn get_host_metrics() -> Json<HostMetrics> {
    let mut cache = METRICS_CACHE.lock().unwrap();

    // Initialize cache on first access
    if cache.is_none() {
        *cache = Some(MetricsCache {
            metrics: None,
            updated_at: Instant::now(),
        });
    }

    let cache = cache.as_mut().unwrap();

    // Return cached value if fresh (< 2 seconds old)
    if cache.updated_at.elapsed() < Duration::from_secs(2) {
        if let Some(ref m) = cache.metrics {
            return Json(m.clone());
        }
    }

    // Refresh metrics
    let mut sys = System::new_all();
    sys.refresh_all();

    let disks = Disks::new_with_refreshed_list();
    let (disk_used, disk_total) = disks
        .iter()
        .filter(|d| d.mount_point() == std::path::Path::new("/"))
        .map(|d| (d.total_space() - d.available_space(), d.total_space()))
        .next()
        .unwrap_or((0, 0));

    let metrics = HostMetrics {
        hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
        cpu_pct: sys.global_cpu_usage(),
        ram_used_bytes: sys.used_memory(),
        ram_total_bytes: sys.total_memory(),
        disk_used_bytes: disk_used,
        disk_total_bytes: disk_total,
    };

    cache.metrics = Some(metrics.clone());
    cache.updated_at = Instant::now();

    Json(metrics)
}

// ===== Handler: GET /api/v1/costs/daily =====

pub async fn get_costs_daily(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Result<Json<Vec<DailyCost>>, StatusCode> {
    // Query events table for CostUpdated events in last 30 days
    // Body is stored as MessagePack, so we need to deserialize and aggregate in Rust
    #[derive(sqlx::FromRow)]
    struct EventRow {
        date: String,
        body: Vec<u8>,
    }

    let rows = sqlx::query_as::<_, EventRow>(
        r#"
        SELECT
            DATE(ts/1000, 'unixepoch') as date,
            body
        FROM events
        WHERE kind = 'AgentEvent'
          AND ts >= (strftime('%s', 'now', '-30 days') * 1000)
        ORDER BY ts ASC
        "#
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(|e| {
        tracing::error!("costs/daily query failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Deserialize events and aggregate by date
    let mut daily_totals: HashMap<String, f64> = HashMap::new();

    for row in rows {
        // Deserialize the MessagePack body
        let event: Event = match rmp_serde::from_slice(&row.body) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to deserialize event: {}", e);
                continue;
            }
        };

        // Extract CostUpdated dollars
        if let Event::AgentEvent { event: agent_event, .. } = event {
            if let AgentEvent::CostUpdated { dollars } = agent_event {
                *daily_totals.entry(row.date.clone()).or_insert(0.0) += dollars;
            }
        }
    }

    // Convert to sorted Vec<DailyCost>
    let mut costs: Vec<DailyCost> = daily_totals
        .into_iter()
        .map(|(date, dollars)| DailyCost { date, dollars })
        .collect();

    costs.sort_by(|a, b| a.date.cmp(&b.date));

    Ok(Json(costs))
}

// ===== Handler: GET /api/v1/costs/by-model =====

pub async fn get_costs_by_model() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

// ===== Handler: GET /api/v1/pipeline/runs =====

pub async fn get_pipeline_runs() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
