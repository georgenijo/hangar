pub mod events;
pub mod output;
pub mod resize;
pub mod sessions;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
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
