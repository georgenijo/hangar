use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{ringbuf::RingBuf, AppState};

#[derive(Deserialize)]
pub struct OutputQuery {
    offset: Option<u64>,
    len: Option<u32>,
}

pub async fn get_output(
    Path(id): Path<String>,
    Query(params): Query<OutputQuery>,
    State(state): State<AppState>,
) -> Response {
    let ring_path = state.ring_dir.join(&id).join("output.bin");

    if !ring_path.exists() {
        return (StatusCode::NOT_FOUND, "session output not found").into_response();
    }

    let ring = match RingBuf::open(&ring_path) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to open ring: {}", e),
            )
                .into_response();
        }
    };

    let len = params.len.unwrap_or(65536);
    let offset = params
        .offset
        .unwrap_or_else(|| ring.head().saturating_sub(len as u64));
    let current_head = ring.head();
    let capacity = ring.capacity();

    match ring.read_at(offset, len) {
        Ok(data) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/octet-stream".to_string()),
                ("X-Ring-Head".parse().unwrap(), current_head.to_string()),
                ("X-Ring-Capacity".parse().unwrap(), capacity.to_string()),
            ],
            data,
        )
            .into_response(),
        Err(e) if e.to_string().contains("overwritten") => {
            let body = json!({
                "error": "data_overwritten",
                "current_head": current_head,
            });
            (StatusCode::GONE, axum::Json(body)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("read error: {}", e),
        )
            .into_response(),
    }
}
