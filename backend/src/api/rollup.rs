use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::events::{AgentEvent, Event};
use crate::AppState;

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

pub async fn get_costs_by_model(
    State(state): State<AppState>,
) -> Result<Json<Vec<ModelCost>>, StatusCode> {
    // Fetch all AgentEvent records from the events table
    #[derive(sqlx::FromRow)]
    struct EventRow {
        session_id: String,
        body: Vec<u8>,
    }

    let rows = sqlx::query_as::<_, EventRow>(
        "SELECT session_id, body FROM events WHERE kind = 'AgentEvent' ORDER BY ts ASC",
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(|e| {
        tracing::error!("costs/by-model query failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Track the most recent model per session and accumulate costs per session
    let mut session_models: HashMap<String, String> = HashMap::new();
    let mut session_costs: HashMap<String, f64> = HashMap::new();

    for row in rows {
        // Deserialize the MessagePack body
        let event: Event = rmp_serde::from_slice(&row.body).map_err(|e| {
            tracing::error!("Failed to deserialize event body: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        // Extract AgentEvent from Event enum
        if let Event::AgentEvent { event: agent_event, .. } = event {
            match agent_event {
                AgentEvent::ModelChanged { model } => {
                    // Update the most recent model for this session
                    session_models.insert(row.session_id.clone(), model);
                }
                AgentEvent::CostUpdated { dollars } => {
                    // Accumulate costs for this session
                    *session_costs.entry(row.session_id.clone()).or_insert(0.0) += dollars;
                }
                _ => {}
            }
        }
    }

    // Aggregate costs by model
    let mut model_costs: HashMap<String, f64> = HashMap::new();

    for (session_id, cost) in session_costs {
        let model = session_models
            .get(&session_id)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        *model_costs.entry(model).or_insert(0.0) += cost;
    }

    // Convert to Vec<ModelCost> and sort by dollars descending
    let mut costs: Vec<ModelCost> = model_costs
        .into_iter()
        .map(|(model, dollars)| ModelCost { model, dollars })
        .collect();

    costs.sort_by(|a, b| b.dollars.partial_cmp(&a.dollars).unwrap_or(std::cmp::Ordering::Equal));

    Ok(Json(costs))
}

// ===== Handler: GET /api/v1/pipeline/runs =====

pub async fn get_pipeline_runs() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
