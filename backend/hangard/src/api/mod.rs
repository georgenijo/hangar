use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::pty::{resize_pty, spawn_pty, start_reader_loop};
use crate::session::{
    validate_slug, CreateSessionRequest, ResizeRequest, Session, SessionHandle, SessionId,
    SessionKind, SessionState, SessionStore,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use dashmap::DashMap;
use serde_json::json;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub sessions: SessionStore,
    pub start_time: Instant,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            sessions: Arc::new(DashMap::new()),
            start_time: Instant::now(),
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/sessions", post(spawn_session))
        .route("/api/v1/sessions", get(list_sessions))
        .route("/api/v1/sessions/{id}/resize", post(resize_session))
        .route("/api/v1/health", get(health_check))
}

async fn spawn_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    if !validate_slug(&req.slug) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "error": "invalid slug",
                "detail": "must match ^[a-z][a-z0-9-]{0,31}$"
            })),
        )
            .into_response();
    }

    let slug_taken = state
        .sessions
        .iter()
        .any(|e| e.value().session.lock().unwrap().slug == req.slug);
    if slug_taken {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error": "slug already exists"})),
        )
            .into_response();
    }

    let command = match &req.kind {
        SessionKind::Shell { command } => command.clone(),
        _ => {
            return (
                StatusCode::NOT_IMPLEMENTED,
                Json(json!({"error": "only Shell kind is supported in this version"})),
            )
                .into_response();
        }
    };

    let cwd = req.cwd.as_deref().map(PathBuf::from).unwrap_or_else(|| {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/home/george"))
    });

    let env: BTreeMap<String, String> = req.env.unwrap_or_default();

    let (master, child) = match spawn_pty(&command, &cwd, &env, 80, 24) {
        Ok(pair) => pair,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("failed to spawn pty: {e}")})),
            )
                .into_response();
        }
    };

    let writer = match master.take_writer() {
        Ok(w) => w,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("failed to get pty writer: {e}")})),
            )
                .into_response();
        }
    };

    let (tx, _) = broadcast::channel::<Vec<u8>>(256);
    start_reader_loop(&*master, tx.clone());

    let id = SessionId::new();
    let now = Utc::now();
    let labels = req.labels.unwrap_or_default();

    let session = Session {
        id: id.to_string(),
        slug: req.slug.clone(),
        kind: req.kind,
        state: SessionState::Idle,
        cwd: cwd.clone(),
        env: env.clone(),
        agent_meta: None,
        labels,
        node_id: "local".into(),
        created_at: now,
        last_activity_at: now,
        exit: None,
    };

    let snapshot = session.clone();

    let handle = Arc::new(SessionHandle {
        session: Mutex::new(session),
        pty_master: Arc::new(Mutex::new(master)),
        pty_writer: Arc::new(Mutex::new(writer)),
        output_tx: tx,
        child: Arc::new(Mutex::new(child)),
    });

    state.sessions.insert(id.to_string(), handle);

    (StatusCode::CREATED, Json(snapshot)).into_response()
}

async fn list_sessions(State(state): State<AppState>) -> impl IntoResponse {
    let sessions: Vec<Session> = state
        .sessions
        .iter()
        .map(|e| {
            let handle = e.value();
            let mut session = handle.session.lock().unwrap().clone();
            if session.state != SessionState::Exited {
                if let Some(status) = handle.try_wait() {
                    session.state = SessionState::Exited;
                    session.exit = Some(crate::session::ExitInfo {
                        code: Some(status.exit_code() as i32),
                        signal: None,
                        exited_at: Utc::now(),
                    });
                }
            }
            session
        })
        .collect();
    Json(sessions)
}

async fn resize_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ResizeRequest>,
) -> impl IntoResponse {
    let handle = find_session(&state.sessions, &id);
    let handle = match handle {
        Some(h) => h,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "session not found"})),
            )
                .into_response();
        }
    };

    let master = handle.pty_master.lock().unwrap();
    match resize_pty(&**master, req.cols, req.rows) {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("resize failed: {e}")})),
        )
            .into_response(),
    }
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_s": uptime
    }))
}

pub fn find_session(store: &SessionStore, id: &str) -> Option<Arc<SessionHandle>> {
    if let Some(h) = store.get(id) {
        return Some(h.clone());
    }
    store
        .iter()
        .find(|e| e.value().session.lock().unwrap().slug == id)
        .map(|e| e.value().clone())
}
