use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use ulid::Ulid;

use crate::sandbox::{SandboxState, SandboxStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionId(String);

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionId {
    pub fn new() -> Self {
        SessionId(Ulid::new().to_string())
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SessionId {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(SessionId(s.to_string()))
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionKind {
    Shell,
    ClaudeCode {
        #[serde(default)]
        config_override: Option<PathBuf>,
        #[serde(default)]
        project_dir: Option<PathBuf>,
    },
    RawBytes,
    Codex {
        #[serde(default)]
        project_dir: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    #[serde(alias = "starting")]
    Booting,
    #[serde(alias = "running", alias = "paused")]
    Idle,
    Streaming,
    Awaiting,
    Error,
    #[serde(alias = "dead", alias = "exiting")]
    Exited,
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("{:?}", self));
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitInfo {
    pub code: Option<i32>,
    pub signal: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMeta {
    pub name: String,
    pub version: Option<String>,
    pub model: Option<String>,
    pub tokens_used: u64,
    pub last_tool_call: Option<String>,
    #[serde(default)]
    pub context_pct: Option<f32>,
    #[serde(default)]
    pub cost_dollars: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub slug: String,
    pub node_id: String,
    pub kind: SessionKind,
    pub state: SessionState,
    pub cwd: String,
    pub env: serde_json::Value,
    pub agent_meta: Option<AgentMeta>,
    pub labels: serde_json::Value,
    pub created_at: i64,
    pub last_activity_at: i64,
    pub exit: Option<ExitInfo>,
    pub sandbox: Option<SandboxStatus>,
}

impl SessionKind {
    pub fn driver_kind(&self) -> &str {
        match self {
            SessionKind::Shell => "shell",
            SessionKind::ClaudeCode { .. } => "claude_code",
            SessionKind::RawBytes => "raw_bytes",
            SessionKind::Codex { .. } => "codex",
        }
    }
}

impl Session {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT id, slug, node_id, kind, state, cwd, env, agent_meta, labels, created_at, last_activity_at, exit, sandbox
             FROM sessions ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(Session::from_row).collect()
    }

    pub async fn insert(&self, pool: &SqlitePool) -> Result<()> {
        let id = self.id.to_string();
        let kind = serde_json::to_string(&self.kind)?;
        let state = serde_json::to_string(&self.state)?;
        let env = serde_json::to_string(&self.env)?;
        let labels = serde_json::to_string(&self.labels)?;
        let agent_meta = self
            .agent_meta
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let exit = self.exit.as_ref().map(serde_json::to_string).transpose()?;
        let sandbox = self
            .sandbox
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        sqlx::query(
            "INSERT INTO sessions (id, slug, node_id, kind, state, cwd, env, agent_meta, labels, created_at, last_activity_at, exit, sandbox)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&self.slug)
        .bind(&self.node_id)
        .bind(&kind)
        .bind(&state)
        .bind(&self.cwd)
        .bind(&env)
        .bind(&agent_meta)
        .bind(&labels)
        .bind(self.created_at)
        .bind(self.last_activity_at)
        .bind(&exit)
        .bind(&sandbox)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get(pool: &SqlitePool, id: &SessionId) -> Result<Option<Session>> {
        let id_str = id.to_string();
        let row = sqlx::query_as::<_, SessionRow>(
            "SELECT id, slug, node_id, kind, state, cwd, env, agent_meta, labels, created_at, last_activity_at, exit, sandbox
             FROM sessions WHERE id = ?",
        )
        .bind(&id_str)
        .fetch_optional(pool)
        .await?;

        row.map(Session::from_row).transpose()
    }

    pub async fn get_by_id_or_slug(pool: &SqlitePool, id_or_slug: &str) -> Result<Option<Session>> {
        let row = sqlx::query_as::<_, SessionRow>(
            "SELECT id, slug, node_id, kind, state, cwd, env, agent_meta, labels, created_at, last_activity_at, exit, sandbox
             FROM sessions WHERE id = ?1 OR (slug = ?1 AND node_id = 'local') LIMIT 1",
        )
        .bind(id_or_slug)
        .fetch_optional(pool)
        .await?;

        row.map(Session::from_row).transpose()
    }

    pub async fn update_state(
        pool: &SqlitePool,
        id: &SessionId,
        state: SessionState,
    ) -> Result<()> {
        let id_str = id.to_string();
        let state_str = serde_json::to_string(&state)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        sqlx::query("UPDATE sessions SET state = ?, last_activity_at = ? WHERE id = ?")
            .bind(&state_str)
            .bind(now)
            .bind(&id_str)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update_agent_meta(
        pool: &SqlitePool,
        id: &SessionId,
        meta: &AgentMeta,
    ) -> Result<()> {
        let id_str = id.to_string();
        let meta_json = serde_json::to_string(meta)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;
        sqlx::query(
            "UPDATE sessions SET agent_meta = ?, last_activity_at = ? WHERE id = ?",
        )
        .bind(&meta_json)
        .bind(now)
        .bind(&id_str)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Hard-delete a session and all events. Removes FTS rows via the FTS5
    /// 'delete' command (external-content tables cannot be updated via plain
    /// DELETE), then removes events (FK), then the sessions row itself.
    /// Returns true if a row was removed from `sessions`.
    ///
    /// Uses `BEGIN IMMEDIATE` to acquire the write lock up-front so the
    /// connection's configured `busy_timeout` governs the contention
    /// window — otherwise sqlx's default deferred transaction only notices
    /// the concurrent writer on its first statement, after our inner
    /// queries have already been submitted.
    pub async fn delete(pool: &SqlitePool, id: &SessionId) -> Result<bool> {
        let id_str = id.to_string();
        let mut conn = pool.acquire().await?;

        sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;
        let result = Self::delete_inner(&mut conn, &id_str).await;
        match result {
            Ok(deleted) => {
                sqlx::query("COMMIT").execute(&mut *conn).await?;
                Ok(deleted)
            }
            Err(e) => {
                let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
                Err(e)
            }
        }
    }

    async fn delete_inner(conn: &mut sqlx::SqliteConnection, id_str: &str) -> Result<bool> {
        // events_fts is external-content (content='events'). A direct
        // `DELETE FROM events_fts` yields SQLITE_CORRUPT_VTAB, so we emit
        // the FTS5 'delete' command for each row belonging to this session.
        // Rows with no body_text have no FTS entry to remove.
        let fts_rows: Vec<(i64, Option<String>, String, String)> = sqlx::query_as(
            "SELECT id, body_text, session_id, kind FROM events WHERE session_id = ?",
        )
        .bind(id_str)
        .fetch_all(&mut *conn)
        .await?;

        for (rowid, body_text, sid, kind) in &fts_rows {
            if let Some(body) = body_text {
                sqlx::query(
                    "INSERT INTO events_fts(events_fts, rowid, body_text, session_id, kind) \
                     VALUES ('delete', ?, ?, ?, ?)",
                )
                .bind(rowid)
                .bind(body)
                .bind(sid)
                .bind(kind)
                .execute(&mut *conn)
                .await?;
            }
        }

        sqlx::query("DELETE FROM events WHERE session_id = ?")
            .bind(id_str)
            .execute(&mut *conn)
            .await?;

        let res = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id_str)
            .execute(&mut *conn)
            .await?;

        Ok(res.rows_affected() > 0)
    }

    pub async fn update_sandbox(pool: &SqlitePool, id: &str, status: &SandboxStatus) -> Result<()> {
        let sandbox_json = serde_json::to_string(status)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;
        sqlx::query("UPDATE sessions SET sandbox = ?, last_activity_at = ? WHERE id = ?")
            .bind(&sandbox_json)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn update_sandbox_state(
        pool: &SqlitePool,
        id: &str,
        state: SandboxState,
    ) -> Result<()> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT sandbox FROM sessions WHERE id = ?")
                .bind(id)
                .fetch_optional(pool)
                .await?;

        let sandbox_json = match row.and_then(|(s,)| s) {
            Some(j) => j,
            None => return Ok(()),
        };

        let mut status: SandboxStatus = match serde_json::from_str(&sandbox_json) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("failed to deserialize sandbox for state update: {e}");
                return Ok(());
            }
        };

        status.state = state;
        let updated_json = serde_json::to_string(&status)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;
        sqlx::query("UPDATE sessions SET sandbox = ?, last_activity_at = ? WHERE id = ?")
            .bind(&updated_json)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn mark_active_as_exited(pool: &SqlitePool) -> Result<u64> {
        let booting = serde_json::to_string(&SessionState::Booting)?;
        let idle = serde_json::to_string(&SessionState::Idle)?;
        let streaming = serde_json::to_string(&SessionState::Streaming)?;
        let exited = serde_json::to_string(&SessionState::Exited)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        let result = sqlx::query(
            "UPDATE sessions SET state = ?, last_activity_at = ? WHERE state = ? OR state = ? OR state = ?",
        )
        .bind(&exited)
        .bind(now)
        .bind(&booting)
        .bind(&idle)
        .bind(&streaming)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    fn from_row(row: SessionRow) -> Result<Session> {
        let kind =
            serde_json::from_str::<SessionKind>(&row.kind).or_else(|_| -> Result<SessionKind> {
                let bare: String = serde_json::from_str(&row.kind)?;
                match bare.as_str() {
                    "shell" => Ok(SessionKind::Shell),
                    "claude_code" => Ok(SessionKind::ClaudeCode {
                        config_override: None,
                        project_dir: None,
                    }),
                    "raw_bytes" => Ok(SessionKind::RawBytes),
                    "codex" => Ok(SessionKind::Codex { project_dir: None }),
                    other => anyhow::bail!("unknown session kind: {other}"),
                }
            })?;

        let agent_meta = row
            .agent_meta
            .as_deref()
            .map(|s| {
                serde_json::from_str::<AgentMeta>(s).or_else(|_| {
                    #[derive(Deserialize)]
                    struct LegacyAgentMeta {
                        model: String,
                        prompt_tokens: u64,
                        completion_tokens: u64,
                    }
                    let legacy: LegacyAgentMeta = serde_json::from_str(s)?;
                    Ok::<_, serde_json::Error>(AgentMeta {
                        name: "unknown".to_string(),
                        version: None,
                        model: Some(legacy.model),
                        tokens_used: legacy.prompt_tokens + legacy.completion_tokens,
                        last_tool_call: None,
                        context_pct: None,
                        cost_dollars: None,
                    })
                })
            })
            .transpose()?;

        let sandbox = row
            .sandbox
            .as_deref()
            .map(|s| {
                serde_json::from_str::<SandboxStatus>(s).map_err(|e| {
                    tracing::warn!("failed to deserialize sandbox column: {e}");
                    e
                })
            })
            .and_then(|r| r.ok());

        Ok(Session {
            id: SessionId(row.id),
            slug: row.slug,
            node_id: row.node_id,
            kind,
            state: serde_json::from_str(&row.state)?,
            cwd: row.cwd,
            env: serde_json::from_str(&row.env)?,
            agent_meta,
            labels: serde_json::from_str(&row.labels)?,
            created_at: row.created_at,
            last_activity_at: row.last_activity_at,
            exit: row.exit.as_deref().map(serde_json::from_str).transpose()?,
            sandbox,
        })
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: String,
    slug: String,
    node_id: String,
    kind: String,
    state: String,
    cwd: String,
    env: String,
    agent_meta: Option<String>,
    labels: String,
    created_at: i64,
    last_activity_at: i64,
    exit: Option<String>,
    sandbox: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_meta_new_fields_default_null_on_legacy() {
        let json = r#"{"name":"claude_code","tokens_used":100}"#;
        let meta: AgentMeta = serde_json::from_str(json).unwrap();
        assert!(meta.context_pct.is_none());
        assert!(meta.cost_dollars.is_none());
    }

    #[test]
    fn test_agent_meta_round_trips_new_fields() {
        let meta = AgentMeta {
            name: "claude_code".to_string(),
            version: None,
            model: Some("claude-sonnet-4".to_string()),
            tokens_used: 1234,
            last_tool_call: Some("Bash".to_string()),
            context_pct: Some(0.42),
            cost_dollars: Some(0.015),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: AgentMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tokens_used, 1234);
        assert!((back.context_pct.unwrap() - 0.42).abs() < 1e-4);
        assert!((back.cost_dollars.unwrap() - 0.015).abs() < 1e-6);
        assert_eq!(back.last_tool_call.as_deref(), Some("Bash"));
    }

    #[tokio::test]
    async fn test_update_agent_meta_persists() {
        let db = crate::db::Db::new_in_memory().await.unwrap();
        let pool = db.pool();

        let s = Session {
            id: SessionId::new(),
            slug: "test-slug".to_string(),
            node_id: "local".to_string(),
            kind: SessionKind::ClaudeCode {
                config_override: None,
                project_dir: None,
            },
            state: SessionState::Idle,
            cwd: "/tmp".to_string(),
            env: serde_json::json!({}),
            agent_meta: None,
            labels: serde_json::json!([]),
            created_at: 0,
            last_activity_at: 0,
            exit: None,
            sandbox: None,
        };
        s.insert(pool).await.unwrap();

        let meta = AgentMeta {
            name: "claude_code".to_string(),
            version: None,
            model: Some("claude-sonnet-4".to_string()),
            tokens_used: 999,
            last_tool_call: Some("Bash".to_string()),
            context_pct: Some(0.5),
            cost_dollars: Some(0.02),
        };
        Session::update_agent_meta(pool, &s.id, &meta).await.unwrap();

        let loaded = Session::get(pool, &s.id).await.unwrap().unwrap();
        let m = loaded.agent_meta.unwrap();
        assert_eq!(m.model.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(m.tokens_used, 999);
        assert!((m.context_pct.unwrap() - 0.5).abs() < 1e-4);
        assert!((m.cost_dollars.unwrap() - 0.02).abs() < 1e-6);
    }
}
