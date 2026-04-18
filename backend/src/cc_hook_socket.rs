use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};

use crate::drivers::OobMessage;
use crate::AppState;

async fn handle_cc_hook(
    Path((session_id, hook_name)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let tx = {
        let channels = state.hook_channels.lock().unwrap();
        channels.get(&session_id).cloned()
    };

    let Some(tx) = tx else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default();
    let msg = OobMessage {
        hook: hook_name,
        ts,
        payload,
    };

    match tx.try_send(msg) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

pub fn hook_route() -> Router<AppState> {
    Router::new().route("/_cc_hook/:session_id/:hook_name", post(handle_cc_hook))
}
