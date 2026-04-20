use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use sysinfo::{Disks, System};

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

pub async fn get_costs_daily() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

// ===== Handler: GET /api/v1/costs/by-model =====

pub async fn get_costs_by_model() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

// ===== Handler: GET /api/v1/pipeline/runs =====

pub async fn get_pipeline_runs() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
