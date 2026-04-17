use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use regex::Regex;
use serde::Deserialize;

use crate::api::{resolve_session, WriterLock};
use crate::drivers::{DriverRegistry, PtyHandle, SpawnRequest};
use crate::events::Event;
use crate::pty;
use crate::session::{Session, SessionId, SessionKind, SessionState};
use crate::AppState;
use sqlx;

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

    let active = if let Some(ref sup) = state.supervisor {
        pty::spawn_pty_supervised(
            sup,
            session_id.clone(),
            spawn_cfg,
            driver,
            state.ring_dir.clone(),
            state.event_bus.clone(),
            state.db.pool().clone(),
            (body.cols, body.rows),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        pty::spawn_pty(
            session_id.clone(),
            spawn_cfg,
            driver,
            state.ring_dir.clone(),
            state.event_bus.clone(),
            state.db.pool().clone(),
            (body.cols, body.rows),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

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

pub async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Session>, StatusCode> {
    let session = resolve_session(&state, &id).await?;
    Ok(Json(session))
}

#[derive(Deserialize)]
pub struct PatchSessionRequest {
    pub slug: Option<String>,
    pub labels: Option<serde_json::Value>,
}

pub async fn patch_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PatchSessionRequest>,
) -> Result<Json<Session>, StatusCode> {
    let session = resolve_session(&state, &id).await?;
    let ulid = session.id.to_string();

    if let Some(ref slug) = body.slug {
        if !slug_re().is_match(slug) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    if let Some(ref labels) = body.labels {
        if !labels.is_object() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let new_slug = body.slug.as_deref().map(|s| s.to_string());
    let new_labels = body.labels.as_ref().map(|v| v.to_string());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let result = sqlx::query(
        "UPDATE sessions SET slug = COALESCE(?1, slug), labels = COALESCE(?2, labels), last_activity_at = ?3 WHERE id = ?4",
    )
    .bind(&new_slug)
    .bind(&new_labels)
    .bind(now)
    .bind(&ulid)
    .execute(state.db.pool())
    .await;

    match result {
        Err(sqlx::Error::Database(dbe)) if dbe.is_unique_violation() => {
            return Err(StatusCode::CONFLICT);
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        Ok(_) => {}
    }

    let updated = Session::get_by_id_or_slug(state.db.pool(), &ulid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(updated))
}

pub async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let session = resolve_session(&state, &id).await?;
    let ulid = session.id.to_string();

    let active = {
        let mut sessions = state.sessions.write().unwrap();
        sessions.remove(&ulid)
    };

    if let Some(active) = active {
        let writer_clone = std::sync::Arc::clone(&active.writer);
        let mut handle = PtyHandle::new(Box::new(WriterLock(writer_clone)));
        let _ = active
            .driver
            .lock()
            .unwrap()
            .shutdown(&mut handle, Duration::from_secs(5));
    }

    if let Some(ref sup) = state.supervisor {
        let _ = sup.kill(&ulid, libc::SIGTERM).await;
    }

    Session::update_state(state.db.pool(), &session.id, SessionState::Exited)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.event_bus.send(
        ulid,
        Event::StateChanged {
            from: session.state,
            to: SessionState::Exited,
        },
    );

    Ok(StatusCode::NO_CONTENT)
}
