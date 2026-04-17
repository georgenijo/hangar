use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    events::EventStore,
    session::{Session, SessionId},
    AppState,
};

#[derive(Deserialize)]
pub struct EventsQuery {
    since: Option<i64>,
    kind: Option<String>,
    limit: Option<i64>,
}

pub async fn get_events(
    Path(id): Path<String>,
    Query(params): Query<EventsQuery>,
    State(state): State<AppState>,
) -> Response {
    let session_id: SessionId = match id.parse() {
        Ok(s) => s,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid session id").into_response(),
    };

    let pool = state.db.pool();

    match Session::get(pool, &session_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, "session not found").into_response(),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("db error: {}", e),
            )
                .into_response()
        }
    }

    let since = params.since.unwrap_or(0);
    let limit = params.limit.unwrap_or(100).min(1000);
    let kind = params.kind.as_deref();

    match EventStore::query(pool, &id, since, kind, limit).await {
        Ok(events) => {
            let json_events: Vec<serde_json::Value> = events
                .into_iter()
                .map(|e| {
                    json!({
                        "id": e.id,
                        "session_id": e.session_id,
                        "ts": e.ts,
                        "kind": e.kind,
                        "event": e.event,
                    })
                })
                .collect();
            Json(json_events).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("query error: {}", e),
        )
            .into_response(),
    }
}
