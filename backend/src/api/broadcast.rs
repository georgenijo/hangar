use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::api::WriterLock;
use crate::drivers::PtyHandle;
use crate::session::Session;
use crate::AppState;

#[derive(Deserialize)]
pub struct BroadcastFilter {
    pub kind: Option<String>,
    pub labels: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Deserialize)]
pub struct BroadcastRequest {
    pub text: String,
    pub filter: Option<BroadcastFilter>,
}

#[derive(Serialize)]
pub struct BroadcastResponse {
    pub sent: usize,
}

pub async fn broadcast(
    State(state): State<AppState>,
    Json(body): Json<BroadcastRequest>,
) -> Result<Json<BroadcastResponse>, StatusCode> {
    let db_sessions = Session::list(state.db.pool())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session_map: std::collections::HashMap<String, Session> = db_sessions
        .into_iter()
        .map(|s| (s.id.to_string(), s))
        .collect();

    let sessions = state.sessions.read().unwrap();
    let mut sent = 0usize;

    for (ulid, active) in sessions.iter() {
        let db_session = match session_map.get(ulid) {
            Some(s) => s,
            None => continue,
        };

        if let Some(ref filter) = body.filter {
            if let Some(ref kind) = filter.kind {
                if db_session.kind.driver_kind() != kind.as_str() {
                    continue;
                }
            }

            if let Some(ref filter_labels) = filter.labels {
                let sess_labels = match db_session.labels.as_object() {
                    Some(m) => m,
                    None => continue,
                };
                let matches = filter_labels
                    .iter()
                    .all(|(k, v)| sess_labels.get(k) == Some(v));
                if !matches {
                    continue;
                }
            }
        }

        let writer_clone = std::sync::Arc::clone(&active.writer);
        let mut handle = PtyHandle::new(Box::new(WriterLock(writer_clone)));

        match active
            .driver
            .lock()
            .unwrap()
            .prompt(&mut handle, &body.text)
        {
            Ok(_) => sent += 1,
            Err(e) => warn!("broadcast failed for session {ulid}: {e}"),
        }
    }

    Ok(Json(BroadcastResponse { sent }))
}
