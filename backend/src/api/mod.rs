pub mod broadcast;
pub mod events;
pub mod key;
pub mod logs;
pub mod metrics;
pub mod output;
pub mod prompt;
pub mod resize;
pub mod search;
pub mod sessions;
pub mod worktree;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::session::Session;
use crate::AppState;

pub struct WriterLock(pub Arc<Mutex<Box<dyn Write + Send>>>);

impl Write for WriterLock {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

pub async fn resolve_session(state: &AppState, id_or_slug: &str) -> Result<Session, StatusCode> {
    Session::get_by_id_or_slug(state.db.pool(), id_or_slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route(
            "/api/v1/sessions",
            post(sessions::create_session).get(sessions::list_sessions),
        )
        .route(
            "/api/v1/sessions/:id",
            get(sessions::get_session)
                .patch(sessions::patch_session)
                .delete(sessions::delete_session),
        )
        .route("/api/v1/sessions/:id/resize", post(resize::resize_session))
        .route("/api/v1/sessions/:id/output", get(output::get_output))
        .route("/api/v1/sessions/:id/events", get(events::get_events))
        .route("/api/v1/sessions/:id/prompt", post(prompt::prompt_session))
        .route("/api/v1/sessions/:id/key", post(key::send_key))
        .route("/api/v1/sessions/:id/fsdiff", get(sessions::get_fs_diff))
        .route(
            "/api/v1/sessions/:id/worktree/tree",
            get(worktree::get_tree),
        )
        .route(
            "/api/v1/sessions/:id/worktree/file",
            get(worktree::get_file),
        )
        .route(
            "/api/v1/sessions/:id/worktree/diff",
            get(worktree::get_diff),
        )
        .route(
            "/api/v1/sessions/:id/merge-overlay",
            post(sessions::merge_overlay_handler),
        )
        .route("/api/v1/search", get(search::search))
        .route("/api/v1/broadcast", post(broadcast::broadcast))
        .route("/api/v1/metrics", get(metrics::get_metrics))
        .route("/api/v1/logs/sources", get(logs::list_sources))
        .route("/ws/v1/sessions/:id/pty", get(crate::ws::pty::ws_pty))
        .route("/ws/v1/logs", get(crate::ws::logs::ws_handler))
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
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_s": state.start_time.elapsed().as_secs(),
        "supervisor_connected": supervisor_connected,
    }))
}
