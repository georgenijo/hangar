use std::collections::HashMap;
use std::sync::OnceLock;

use axum::{extract::State, http::StatusCode, Json};
use regex::Regex;
use serde::Deserialize;

use crate::drivers::{DriverRegistry, SpawnRequest};
use crate::events::Event;
use crate::pty;
use crate::session::{Session, SessionId, SessionKind, SessionState};
use crate::AppState;

static SLUG_RE: OnceLock<Regex> = OnceLock::new();

fn slug_re() -> &'static Regex {
    SLUG_RE.get_or_init(|| Regex::new(r"^[a-z][a-z0-9-]{0,31}$").unwrap())
}

fn default_cols() -> u16 {
    80
}

fn default_rows() -> u16 {
    24
}

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub slug: String,
    pub kind: SessionKind,
    #[serde(default = "default_cols")]
    pub cols: u16,
    #[serde(default = "default_rows")]
    pub rows: u16,
}

pub async fn create_session(
    State(state): State<AppState>,
    Json(body): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<Session>), StatusCode> {
    if !slug_re().is_match(&body.slug) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let registry = DriverRegistry::new();
    let driver_kind = body.kind.driver_kind();
    let driver = registry
        .create(driver_kind)
        .ok_or(StatusCode::BAD_REQUEST)?;

    let cwd = std::env::current_dir()
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")));

    let session_id = SessionId::new();
    let spawn_req = SpawnRequest {
        session_id: session_id.clone(),
        cwd: cwd.clone(),
        env: HashMap::new(),
        kind: body.kind.clone(),
        hmac_key: vec![],
    };

    let spawn_cfg = driver
        .spawn_cfg(&spawn_req)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let session = Session {
        id: session_id.clone(),
        slug: body.slug.clone(),
        node_id: "local".to_string(),
        kind: body.kind,
        state: SessionState::Booting,
        cwd: spawn_cfg.cwd.display().to_string(),
        env: serde_json::json!({}),
        agent_meta: None,
        labels: serde_json::json!([]),
        created_at: now,
        last_activity_at: now,
        exit: None,
    };

    session.insert(state.db.pool()).await.map_err(|e| {
        if let Some(sqlx::Error::Database(dbe)) = e.downcast_ref::<sqlx::Error>() {
            if dbe.is_unique_violation() {
                return StatusCode::CONFLICT;
            }
        }
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let active = pty::spawn_pty(
        session_id.clone(),
        spawn_cfg,
        driver,
        state.ring_dir.clone(),
        state.event_bus.clone(),
        state.db.pool().clone(),
        (body.cols, body.rows),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Session::update_state(state.db.pool(), &session_id, SessionState::Idle)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state
        .event_bus
        .send(session_id.to_string(), Event::SessionCreated);

    state
        .sessions
        .write()
        .unwrap()
        .insert(session_id.to_string(), active);

    let mut response_session = session;
    response_session.state = SessionState::Idle;

    Ok((StatusCode::CREATED, Json(response_session)))
}

pub async fn list_sessions(
    State(state): State<AppState>,
) -> Result<Json<Vec<Session>>, StatusCode> {
    Session::list(state.db.pool())
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
