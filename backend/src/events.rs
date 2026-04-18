use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tracing::info;

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
    SandboxStateChanged {
        state: crate::sandbox::SandboxState,
    },
    SandboxMerged {
        snapshot_id: String,
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
    OverlayDiffReady {
        session_id: SessionId,
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
            Event::OverlayDiffReady { .. } => "OverlayDiffReady",
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

pub fn extract_searchable_text_pub(event: &Event) -> Option<String> {
    extract_searchable_text(event)
}

fn extract_searchable_text(event: &Event) -> Option<String> {
    match event {
        Event::AgentEvent { event, .. } => match event {
            AgentEvent::TurnStarted { content_start, .. } => content_start.clone(),
            AgentEvent::TurnFinished { .. } => None,
            AgentEvent::ThinkingBlock { .. } => None,
            AgentEvent::ToolCallStarted {
                tool, args_preview, ..
            } => Some(format!("{tool} {args_preview}")),
            AgentEvent::ToolCallFinished { result_preview, .. } => {
                if result_preview.is_empty() {
                    None
                } else {
                    Some(result_preview.clone())
                }
            }
            AgentEvent::AwaitingPermission { tool, prompt } => Some(format!("{tool} {prompt}")),
            AgentEvent::ModelChanged { model } => Some(model.clone()),
            AgentEvent::Error { message } => Some(message.clone()),
            AgentEvent::ContextWindowSizeChanged { .. } => None,
        },
        Event::SessionCreated
        | Event::StateChanged { .. }
        | Event::OutputAppended { .. }
        | Event::InputReceived { .. }
        | Event::Resized { .. }
        | Event::MetricsUpdated => None,
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub event_id: i64,
    pub session_id: String,
    pub ts: i64,
    pub kind: String,
    pub snippet: String,
    pub score: f64,
}

#[derive(sqlx::FromRow)]
struct SearchRow {
    id: i64,
    session_id: String,
    ts: i64,
    kind: String,
    snippet: String,
    rank: f64,
}

pub struct EventStore;

impl EventStore {
    pub async fn insert(pool: &SqlitePool, session_id: &str, event: &Event) -> Result<i64> {
        let kind = event.kind_str();
        let body = rmp_serde::to_vec(event)?;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;
        let body_text = extract_searchable_text(event);

        let mut tx = pool.begin().await?;

        let result = sqlx::query(
            "INSERT INTO events (session_id, ts, kind, body, body_text) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(session_id)
        .bind(ts)
        .bind(kind)
        .bind(&body)
        .bind(&body_text)
        .execute(&mut *tx)
        .await?;

        let rowid = result.last_insert_rowid();

        if let Some(ref text) = body_text {
            sqlx::query(
                "INSERT INTO events_fts(rowid, body_text, session_id, kind) VALUES (?, ?, ?, ?)",
            )
            .bind(rowid)
            .bind(text)
            .bind(session_id)
            .bind(kind)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(rowid)
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

    pub async fn backfill_fts(pool: &SqlitePool) -> Result<u64> {
        let mut total: u64 = 0;

        loop {
            let rows: Vec<BackfillRow> = sqlx::query_as::<_, BackfillRow>(
                "SELECT id, session_id, kind, body FROM events WHERE body_text IS NULL LIMIT 500",
            )
            .fetch_all(pool)
            .await?;

            if rows.is_empty() {
                break;
            }

            let mut tx = pool.begin().await?;

            for row in &rows {
                let event: Event = match rmp_serde::from_slice(&row.body) {
                    Ok(e) => e,
                    Err(_) => {
                        sqlx::query("UPDATE events SET body_text = '' WHERE id = ?")
                            .bind(row.id)
                            .execute(&mut *tx)
                            .await?;
                        continue;
                    }
                };

                let text = extract_searchable_text(&event);

                match &text {
                    Some(t) => {
                        sqlx::query("UPDATE events SET body_text = ? WHERE id = ?")
                            .bind(t)
                            .bind(row.id)
                            .execute(&mut *tx)
                            .await?;
                        sqlx::query(
                            "INSERT INTO events_fts(rowid, body_text, session_id, kind) VALUES (?, ?, ?, ?)",
                        )
                        .bind(row.id)
                        .bind(t)
                        .bind(&row.session_id)
                        .bind(&row.kind)
                        .execute(&mut *tx)
                        .await?;
                    }
                    None => {
                        sqlx::query("UPDATE events SET body_text = '' WHERE id = ?")
                            .bind(row.id)
                            .execute(&mut *tx)
                            .await?;
                    }
                }

                total += 1;
                if total.is_multiple_of(1000) {
                    info!("backfill_fts: processed {} events", total);
                }
            }

            tx.commit().await?;
        }

        Ok(total)
    }

    pub async fn search(
        pool: &SqlitePool,
        query: &str,
        session_ids: Option<&[&str]>,
        kinds: Option<&[&str]>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let mut sql = String::from(
            "SELECT e.id, e.session_id, e.ts, e.kind, \
             snippet(events_fts, 0, '<mark>', '</mark>', '…', 40) as snippet, \
             rank \
             FROM events_fts \
             JOIN events e ON events_fts.rowid = e.id \
             WHERE events_fts MATCH ?",
        );

        if let Some(ids) = session_ids {
            if !ids.is_empty() {
                let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                sql.push_str(&format!(" AND e.session_id IN ({placeholders})"));
            }
        }

        if let Some(ks) = kinds {
            if !ks.is_empty() {
                let placeholders = ks.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                sql.push_str(&format!(" AND e.kind IN ({placeholders})"));
            }
        }

        sql.push_str(" ORDER BY rank LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, SearchRow>(&sql).bind(query);

        if let Some(ids) = session_ids {
            for id in ids {
                q = q.bind(*id);
            }
        }

        if let Some(ks) = kinds {
            for k in ks {
                q = q.bind(*k);
            }
        }

        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(pool).await.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("fts5") || msg.contains("malformed") || msg.contains("syntax") {
                SearchError::BadQuery(msg)
            } else {
                SearchError::Db(anyhow::anyhow!(msg))
            }
        })?;

        Ok(rows
            .into_iter()
            .map(|r| SearchResult {
                event_id: r.id,
                session_id: r.session_id,
                ts: r.ts,
                kind: r.kind,
                snippet: r.snippet,
                score: r.rank,
            })
            .collect())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("invalid search query: {0}")]
    BadQuery(String),
    #[error("database error: {0}")]
    Db(#[from] anyhow::Error),
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: i64,
    session_id: String,
    ts: i64,
    kind: String,
    body: Vec<u8>,
}

#[derive(sqlx::FromRow)]
struct BackfillRow {
    id: i64,
    session_id: String,
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
