use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_s: u64,
    pub supervisor_connected: bool,
}

pub async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime_s = state.started_at.elapsed().as_secs();
    let supervisor_connected = state.supervisor_connected.load(Ordering::SeqCst);
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_s,
        supervisor_connected,
    })
}
