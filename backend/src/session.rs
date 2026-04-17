use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fmt;
use std::str::FromStr;
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionId(String);

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
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Shell,
    ClaudeCode,
    RawBytes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Starting,
    Running,
    Paused,
    Exiting,
    Dead,
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
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
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
}

impl Session {
    pub async fn insert(&self, pool: &SqlitePool) -> Result<()> {
        let id = self.id.to_string();
        let kind = serde_json::to_string(&self.kind)?;
        let state = serde_json::to_string(&self.state)?;
        let env = serde_json::to_string(&self.env)?;
        let labels = serde_json::to_string(&self.labels)?;
        let agent_meta = self
            .agent_meta
            .as_ref()
            .map(|m| serde_json::to_string(m))
            .transpose()?;
        let exit = self
            .exit
            .as_ref()
            .map(|e| serde_json::to_string(e))
            .transpose()?;

        sqlx::query(
            "INSERT INTO sessions (id, slug, node_id, kind, state, cwd, env, agent_meta, labels, created_at, last_activity_at, exit)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get(pool: &SqlitePool, id: &SessionId) -> Result<Option<Session>> {
        let id_str = id.to_string();
        let row = sqlx::query_as::<_, SessionRow>(
            "SELECT id, slug, node_id, kind, state, cwd, env, agent_meta, labels, created_at, last_activity_at, exit
             FROM sessions WHERE id = ?",
        )
        .bind(&id_str)
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

        sqlx::query(
            "UPDATE sessions SET state = ?, last_activity_at = ? WHERE id = ?",
        )
        .bind(&state_str)
        .bind(now)
        .bind(&id_str)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn mark_running_as_dead(pool: &SqlitePool) -> Result<u64> {
        let running = serde_json::to_string(&SessionState::Running)?;
        let starting = serde_json::to_string(&SessionState::Starting)?;
        let dead = serde_json::to_string(&SessionState::Dead)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        let result = sqlx::query(
            "UPDATE sessions SET state = ?, last_activity_at = ? WHERE state = ? OR state = ?",
        )
        .bind(&dead)
        .bind(now)
        .bind(&running)
        .bind(&starting)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    fn from_row(row: SessionRow) -> Result<Session> {
        Ok(Session {
            id: SessionId(row.id),
            slug: row.slug,
            node_id: row.node_id,
            kind: serde_json::from_str(&row.kind)?,
            state: serde_json::from_str(&row.state)?,
            cwd: row.cwd,
            env: serde_json::from_str(&row.env)?,
            agent_meta: row
                .agent_meta
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?,
            labels: serde_json::from_str(&row.labels)?,
            created_at: row.created_at,
            last_activity_at: row.last_activity_at,
            exit: row.exit.as_deref().map(serde_json::from_str).transpose()?,
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
}
