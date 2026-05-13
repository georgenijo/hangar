use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use tracing::error;

use crate::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DailyRow {
    pub date: String,
    pub usd: f64,
    pub tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct DailyResponse {
    pub days: Vec<DailyRow>,
}

pub async fn costs_daily(
    State(state): State<AppState>,
) -> Result<Json<DailyResponse>, StatusCode> {
    let rows = sqlx::query_as::<_, DailyRow>(
        r#"
        SELECT
            strftime('%Y-%m-%d', created_at / 1000, 'unixepoch') AS date,
            COALESCE(SUM(CAST(json_extract(agent_meta, '$.cost_dollars') AS REAL)), 0.0) AS usd,
            COALESCE(SUM(CAST(json_extract(agent_meta, '$.tokens_used') AS INTEGER)), 0) AS tokens
        FROM sessions
        WHERE agent_meta IS NOT NULL
        GROUP BY date
        ORDER BY date DESC
        "#,
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(|e| {
        error!("costs_daily query failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(DailyResponse { days: rows }))
}

#[derive(Debug, sqlx::FromRow)]
struct ModelRow {
    model: String,
    usd: f64,
}

#[derive(Debug, Serialize)]
pub struct ModelEntry {
    pub model: String,
    pub usd: f64,
    pub share: f64,
}

#[derive(Debug, Serialize)]
pub struct ByModelResponse {
    pub models: Vec<ModelEntry>,
}

pub async fn costs_by_model(
    State(state): State<AppState>,
) -> Result<Json<ByModelResponse>, StatusCode> {
    let rows = sqlx::query_as::<_, ModelRow>(
        r#"
        SELECT
            json_extract(agent_meta, '$.model') AS model,
            COALESCE(SUM(CAST(json_extract(agent_meta, '$.cost_dollars') AS REAL)), 0.0) AS usd
        FROM sessions
        WHERE agent_meta IS NOT NULL
          AND json_extract(agent_meta, '$.model') IS NOT NULL
          AND json_extract(agent_meta, '$.model') != 'null'
        GROUP BY model
        ORDER BY usd DESC
        "#,
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(|e| {
        error!("costs_by_model query failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total: f64 = rows.iter().map(|r| r.usd).sum();

    let models = rows
        .into_iter()
        .map(|r| {
            let share = if total > 0.0 {
                (r.usd / total * 100.0).min(100.0)
            } else {
                0.0
            };
            ModelEntry {
                model: r.model,
                usd: r.usd,
                share,
            }
        })
        .collect();

    Ok(Json(ByModelResponse { models }))
}
