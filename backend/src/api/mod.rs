pub mod events;
pub mod output;

use axum::{routing::get, Router};

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/sessions/:id/output", get(output::get_output))
        .route("/api/v1/sessions/:id/events", get(events::get_events))
        .with_state(state)
}
