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
