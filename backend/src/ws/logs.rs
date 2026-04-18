use std::collections::HashSet;

use axum::extract::ws::Message;
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::Deserialize;
use tokio::sync::broadcast;
use tracing::warn;

use crate::logs::LogLine;
use crate::AppState;

#[derive(Deserialize)]
pub struct LogsParams {
    pub sources: Option<String>,
    pub tail: Option<usize>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<LogsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_logs_ws(socket, state, params))
}

async fn handle_logs_ws(
    mut socket: axum::extract::ws::WebSocket,
    state: AppState,
    params: LogsParams,
) {
    let mut filter: HashSet<String> = params
        .sources
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    let tail_count = params.tail.unwrap_or(state.logs.tail_lines);

    // Phase 1: initial tail
    let sources_to_tail: Vec<String> = if filter.is_empty() {
        state
            .logs
            .sources()
            .iter()
            .map(|s| s.name.clone())
            .collect()
    } else {
        state
            .logs
            .sources()
            .iter()
            .filter(|s| filter.contains(&s.name))
            .map(|s| s.name.clone())
            .collect()
    };

    let mut all_initial: Vec<LogLine> = Vec::new();
    for source_name in &sources_to_tail {
        let lines = state.logs.initial_tail(source_name, tail_count).await;
        all_initial.extend(lines);
    }
    all_initial.sort_by_key(|l| l.ts_us);

    for line in &all_initial {
        let json = log_line_to_json(line);
        if socket.send(Message::Text(json)).await.is_err() {
            return;
        }
    }

    let complete = serde_json::json!({"type": "initial_tail_complete"}).to_string();
    if socket.send(Message::Text(complete)).await.is_err() {
        return;
    }

    // Phase 2: live stream
    let mut rx = state.logs.subscribe();

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(line) => {
                        if filter.is_empty() || filter.contains(&line.source) {
                            let json = log_line_to_json(&line);
                            if socket.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("logs ws lagged by {n}");
                        let msg = serde_json::json!({"type": "lagged", "dropped": n}).to_string();
                        if socket.send(Message::Text(msg)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                            if v.get("type").and_then(|t| t.as_str()) == Some("set_sources") {
                                if let Some(arr) = v.get("sources").and_then(|s| s.as_array()) {
                                    filter = arr
                                        .iter()
                                        .filter_map(|s| s.as_str())
                                        .map(|s| s.to_string())
                                        .collect();
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

fn log_line_to_json(line: &LogLine) -> String {
    serde_json::json!({
        "type": "log",
        "source": line.source,
        "ts_us": line.ts_us,
        "level": line.level,
        "body": line.body,
        "unit": line.unit,
    })
    .to_string()
}
