use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use regex::Regex;
use serde::Deserialize;

use crate::api::{resolve_session, WriterLock};
use crate::drivers::{DriverRegistry, PtyHandle, SpawnRequest};
use crate::events::{AgentEvent, Event};
use crate::pty;
use crate::sandbox::{FsDiffResponse, SandboxSpec, SandboxState};
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
    pub sandbox: Option<SandboxSpec>,
}

#[derive(Deserialize)]
pub struct FsDiffParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
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

    // Set up sandbox if requested
    let sandbox_status = if let Some(ref spec) = body.sandbox {
        let mgr = state
            .sandbox_manager
            .as_ref()
            .ok_or(StatusCode::BAD_REQUEST)?;
        let project_dir = spawn_cfg.cwd.clone();
        let status = mgr
            .create_container(&session_id, spec, &project_dir)
            .await
            .map_err(|e| {
                tracing::error!("sandbox create_container failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        Some(status)
    } else {
        None
    };

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
        sandbox: sandbox_status.clone(),
    };

    session.insert(state.db.pool()).await.map_err(|e| {
        if let Some(sqlx::Error::Database(dbe)) = e.downcast_ref::<sqlx::Error>() {
            if dbe.is_unique_violation() {
                return StatusCode::CONFLICT;
            }
        }
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let active = if let Some(ref sb_status) = sandbox_status {
        pty::spawn_pty_sandboxed(
            session_id.clone(),
            spawn_cfg,
            sb_status,
            driver,
            state.ring_dir.clone(),
            state.event_bus.clone(),
            state.db.pool().clone(),
            (body.cols, body.rows),
        )
        .await
        .map_err(|e| {
            tracing::error!("spawn_pty_sandboxed failed: {e}");
            // Best-effort cleanup
            let mgr = state.sandbox_manager.clone();
            let sid = session_id.clone();
            tokio::spawn(async move {
                if let Some(m) = mgr {
                    let _ = m.stop_container(&sid).await;
                }
            });
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else if let Some(ref sup) = state.supervisor {
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

    // Wire up CC hook channel so cc_hook_socket can deliver hook bodies to the driver.
    {
        let (oob_tx, mut oob_rx) =
            tokio::sync::mpsc::channel::<crate::drivers::OobMessage>(64);
        state
            .hook_channels
            .lock()
            .unwrap()
            .insert(session_id.to_string(), oob_tx);

        let driver = active.driver.clone();
        let event_bus = state.event_bus.clone();
        let hook_channels = state.hook_channels.clone();
        let sid = session_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = oob_rx.recv().await {
                let events = driver.lock().unwrap().on_oob(msg);
                for evt in events {
                    event_bus.send(
                        sid.to_string(),
                        Event::AgentEvent {
                            id: sid.clone(),
                            event: evt,
                        },
                    );
                }
            }
            hook_channels.lock().unwrap().remove(&sid.to_string());
        });
    }

    state
        .sessions
        .write()
        .unwrap()
        .insert(session_id.to_string(), active);

    let mut response_session = session;
    response_session.state = SessionState::Idle;
    response_session.sandbox = sandbox_status;

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

pub async fn get_fs_diff(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<FsDiffParams>,
) -> Result<Json<FsDiffResponse>, StatusCode> {
    let session = resolve_session(&state, &id).await?;

    let sandbox_status = session.sandbox.ok_or(StatusCode::BAD_REQUEST)?;

    if matches!(
        sandbox_status.state,
        SandboxState::Merged | SandboxState::Failed { .. }
    ) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mgr = state
        .sandbox_manager
        .as_ref()
        .ok_or(StatusCode::BAD_REQUEST)?;

    let resp = mgr
        .get_fs_diff(
            &sandbox_status,
            params.limit.unwrap_or(500),
            params.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| {
            tracing::error!("get_fs_diff failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(resp))
}

pub async fn merge_overlay_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Session>, StatusCode> {
    let session = resolve_session(&state, &id).await?;
    let session_id = session.id.clone();
    let ulid = session_id.to_string();

    let sandbox_status = match session.sandbox {
        Some(ref sb) if matches!(sb.state, SandboxState::Running | SandboxState::Stopped) => {
            sb.clone()
        }
        Some(_) | None => return Err(StatusCode::BAD_REQUEST),
    };

    let mgr = state
        .sandbox_manager
        .as_ref()
        .ok_or(StatusCode::BAD_REQUEST)?
        .clone();

    // Mark as merging
    Session::update_sandbox_state(state.db.pool(), &ulid, SandboxState::Merging)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Shutdown active session if running
    if matches!(sandbox_status.state, SandboxState::Running) {
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
    } else {
        state.sessions.write().unwrap().remove(&ulid);
    }

    // Stop container and unmount overlay
    if let Err(e) = mgr.stop_container(&session_id).await {
        tracing::error!("stop_container failed during merge: {e}");
        let _ = Session::update_sandbox_state(
            state.db.pool(),
            &ulid,
            SandboxState::Failed {
                message: e.to_string(),
            },
        )
        .await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Apply overlay to host + restic backup
    let snapshot_id = match mgr.merge_overlay(&sandbox_status).await {
        Ok(sid) => sid,
        Err(e) => {
            tracing::error!("merge_overlay failed: {e}");
            let _ = Session::update_sandbox_state(
                state.db.pool(),
                &ulid,
                SandboxState::Failed {
                    message: e.to_string(),
                },
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Cleanup overlay dirs
    if let Err(e) = mgr.cleanup_overlay_dirs(&session_id) {
        tracing::warn!("cleanup_overlay_dirs failed (non-fatal): {e}");
    }

    // Update DB
    Session::update_sandbox_state(state.db.pool(), &ulid, SandboxState::Merged)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Session::update_state(state.db.pool(), &session_id, SessionState::Exited)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.event_bus.send(
        ulid.clone(),
        Event::AgentEvent {
            id: session_id.clone(),
            event: AgentEvent::SandboxMerged { snapshot_id },
        },
    );

    let updated = Session::get_by_id_or_slug(state.db.pool(), &ulid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(updated))
}
