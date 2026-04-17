use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::session::{SessionId, SessionState};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    TurnStarted {
        turn_id: u64,
        role: TurnRole,
        content_start: Option<String>,
    },
    TurnFinished {
        turn_id: u64,
        tokens_used: u32,
        duration_ms: u32,
    },
    ThinkingBlock {
        turn_id: u64,
        len_chars: u32,
    },
    ToolCallStarted {
        turn_id: u64,
        call_id: String,
        tool: String,
        args_preview: String,
    },
    ToolCallFinished {
        turn_id: u64,
        call_id: String,
        ok: bool,
        result_preview: String,
    },
    AwaitingPermission {
        tool: String,
        prompt: String,
    },
    ModelChanged {
        model: String,
    },
    Error {
        message: String,
    },
    ContextWindowSizeChanged {
        pct_used: f32,
        tokens: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    SessionCreated,
    StateChanged {
        from: SessionState,
        to: SessionState,
    },
    OutputAppended {
        offset: u64,
        len: u32,
    },
    InputReceived {
        data: Vec<u8>,
    },
    Resized {
        cols: u16,
        rows: u16,
    },
    MetricsUpdated,
    AgentEvent {
        id: SessionId,
        event: AgentEvent,
    },
}

impl Event {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Event::SessionCreated => "SessionCreated",
            Event::StateChanged { .. } => "StateChanged",
            Event::OutputAppended { .. } => "OutputAppended",
            Event::InputReceived { .. } => "InputReceived",
            Event::Resized { .. } => "Resized",
            Event::MetricsUpdated => "MetricsUpdated",
            Event::AgentEvent { .. } => "AgentEvent",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub id: i64,
    pub session_id: String,
    pub ts: i64,
    pub kind: String,
    pub event: Event,
}

pub struct EventStore;

impl EventStore {
    pub async fn insert(pool: &SqlitePool, session_id: &str, event: &Event) -> Result<i64> {
        let kind = event.kind_str();
        let body = rmp_serde::to_vec(event)?;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        let result =
            sqlx::query("INSERT INTO events (session_id, ts, kind, body) VALUES (?, ?, ?, ?)")
                .bind(session_id)
                .bind(ts)
                .bind(kind)
                .bind(&body)
                .execute(pool)
                .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn query(
        pool: &SqlitePool,
        session_id: &str,
        since_ms: i64,
        kind: Option<&str>,
        limit: i64,
    ) -> Result<Vec<StoredEvent>> {
        let rows: Vec<EventRow> = if let Some(k) = kind {
            sqlx::query_as::<_, EventRow>(
                "SELECT id, session_id, ts, kind, body FROM events
                 WHERE session_id = ? AND ts >= ? AND kind = ?
                 ORDER BY ts ASC LIMIT ?",
            )
            .bind(session_id)
            .bind(since_ms)
            .bind(k)
            .bind(limit)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, EventRow>(
                "SELECT id, session_id, ts, kind, body FROM events
                 WHERE session_id = ? AND ts >= ?
                 ORDER BY ts ASC LIMIT ?",
            )
            .bind(session_id)
            .bind(since_ms)
            .bind(limit)
            .fetch_all(pool)
            .await?
        };

        rows.into_iter()
            .map(|row| {
                let event: Event = rmp_serde::from_slice(&row.body)?;
                Ok(StoredEvent {
                    id: row.id,
                    session_id: row.session_id,
                    ts: row.ts,
                    kind: row.kind,
                    event,
                })
            })
            .collect()
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: i64,
    session_id: String,
    ts: i64,
    kind: String,
    body: Vec<u8>,
}

pub struct EventBus {
    tx: broadcast::Sender<(String, Event)>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        EventBus { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<(String, Event)> {
        self.tx.subscribe()
    }

    pub fn send(&self, session_id: String, event: Event) {
        let _ = self.tx.send((session_id, event));
    }
}
