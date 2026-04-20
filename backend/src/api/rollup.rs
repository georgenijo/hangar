use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};

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

// ===== Handler: GET /api/v1/metrics/host =====

pub async fn get_host_metrics() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
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
