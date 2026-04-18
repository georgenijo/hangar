use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::{
    events::{EventStore, SearchError},
    AppState,
};

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub session_ids: Option<String>,
    pub kinds: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct SearchResultJson {
    pub event_id: i64,
    pub session_id: String,
    pub ts: i64,
    pub kind: String,
    pub snippet: String,
    pub score: f64,
}

static TAG_RE: OnceLock<Regex> = OnceLock::new();

fn sanitize_snippet(s: &str) -> String {
    let re = TAG_RE.get_or_init(|| Regex::new(r"<[^>]*>").unwrap());
    re.replace_all(s, |caps: &regex::Captures| {
        let tag = &caps[0];
        if tag == "<mark>" || tag == "</mark>" {
            tag.to_string()
        } else {
            String::new()
        }
    })
    .into_owned()
}

pub async fn search(Query(params): Query<SearchQuery>, State(state): State<AppState>) -> Response {
    if params.q.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "query parameter 'q' is required").into_response();
    }

    let session_id_strings: Vec<String> = params
        .session_ids
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    let kind_strings: Vec<String> = params
        .kinds
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    let session_id_refs: Vec<&str> = session_id_strings.iter().map(String::as_str).collect();
    let kind_refs: Vec<&str> = kind_strings.iter().map(String::as_str).collect();

    let session_ids_opt = if session_id_refs.is_empty() {
        None
    } else {
        Some(session_id_refs.as_slice())
    };

    let kinds_opt = if kind_refs.is_empty() {
        None
    } else {
        Some(kind_refs.as_slice())
    };

    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    match EventStore::search(
        state.db.pool(),
        &params.q,
        session_ids_opt,
        kinds_opt,
        limit,
        offset,
    )
    .await
    {
        Ok(results) => {
            let json_results: Vec<SearchResultJson> = results
                .into_iter()
                .map(|r| SearchResultJson {
                    event_id: r.event_id,
                    session_id: r.session_id,
                    ts: r.ts,
                    kind: r.kind,
                    snippet: sanitize_snippet(&r.snippet),
                    score: r.score,
                })
                .collect();
            Json(json_results).into_response()
        }
        Err(SearchError::BadQuery(msg)) => (
            StatusCode::BAD_REQUEST,
            format!("invalid search syntax: {msg}"),
        )
            .into_response(),
        Err(SearchError::Db(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("search error: {e}"),
        )
            .into_response(),
    }
}
