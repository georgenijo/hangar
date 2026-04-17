use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::events::Event;
use crate::AppState;

#[derive(Deserialize)]
pub struct ResizeRequest {
    pub cols: u16,
    pub rows: u16,
}

pub async fn resize_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ResizeRequest>,
) -> Result<StatusCode, StatusCode> {
    let sessions = state.sessions.read().unwrap();
    let active = sessions.get(&id).ok_or(StatusCode::NOT_FOUND)?;

    active
        .master
        .resize(body.cols, body.rows)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.event_bus.send(
        id,
        Event::Resized {
            cols: body.cols,
            rows: body.rows,
        },
    );

    Ok(StatusCode::OK)
}
