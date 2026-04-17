use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::Deserialize;

use crate::drivers::OobMessage;
use crate::AppState;

#[derive(Deserialize)]
struct HookBody {
    hook: String,
    #[serde(default)]
    ts: String,
    #[serde(default)]
    payload: serde_json::Value,
}

async fn handle_cc_hook(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<HookBody>,
) -> impl IntoResponse {
    let tx = {
        let channels = state.hook_channels.lock().unwrap();
        channels.get(&session_id).cloned()
    };

    let Some(tx) = tx else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let msg = OobMessage {
        hook: body.hook,
        ts: body.ts,
        payload: body.payload,
    };

    match tx.try_send(msg) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

pub fn hook_route() -> Router<AppState> {
    Router::new().route("/_cc_hook/:session_id", post(handle_cc_hook))
}
