pub mod health;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/api/v1/health", get(health::health_handler))
}
