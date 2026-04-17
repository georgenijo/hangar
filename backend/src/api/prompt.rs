use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::{resolve_session, WriterLock};
use crate::drivers::PtyHandle;
use crate::AppState;

#[derive(Deserialize)]
pub struct PromptRequest {
    pub text: String,
}

pub async fn prompt_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PromptRequest>,
) -> Result<StatusCode, StatusCode> {
    let session = resolve_session(&state, &id).await?;
    let ulid = session.id.to_string();

    let sessions = state.sessions.read().unwrap();
    let active = sessions.get(&ulid).ok_or(StatusCode::NOT_FOUND)?;

    let writer_clone = std::sync::Arc::clone(&active.writer);
    let mut handle = PtyHandle::new(Box::new(WriterLock(writer_clone)));

    active
        .driver
        .lock()
        .unwrap()
        .prompt(&mut handle, &body.text)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
