pub mod events;
pub mod output;
pub mod resize;
pub mod sessions;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route(
            "/api/v1/sessions",
            post(sessions::create_session).get(sessions::list_sessions),
        )
        .route("/api/v1/sessions/:id/resize", post(resize::resize_session))
        .route("/api/v1/sessions/:id/output", get(output::get_output))
        .route("/api/v1/sessions/:id/events", get(events::get_events))
        .route("/ws/v1/sessions/:id/pty", get(crate::ws::pty::ws_pty))
        .merge(crate::cc_hook_socket::hook_route())
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let supervisor_connected = state
        .supervisor
        .as_ref()
        .map(|s| s.is_connected())
        .unwrap_or(false);
    Json(serde_json::json!({
        "ok": true,
        "supervisor_connected": supervisor_connected,
    }))
}
