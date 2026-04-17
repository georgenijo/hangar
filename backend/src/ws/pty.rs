use std::io::Write;
use std::sync::{Arc, Mutex};

use axum::{
    extract::{ws::Message, Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use tokio::sync::broadcast;

use crate::events::{Event, EventBus};
use crate::pty::PtyMaster;
use crate::AppState;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ControlMessage {
    Resize { cols: u16, rows: u16 },
}

pub async fn ws_pty(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> axum::response::Response {
    let (output_tx, writer, master) = {
        let sessions = state.sessions.read().unwrap();
        match sessions.get(&id) {
            None => return StatusCode::NOT_FOUND.into_response(),
            Some(s) => (s.output_tx.clone(), Arc::clone(&s.writer), s.master.clone()),
        }
    };

    ws.on_upgrade(move |socket| {
        handle_pty_ws(
            socket,
            output_tx.subscribe(),
            writer,
            master,
            state.event_bus,
            id,
        )
    })
    .into_response()
}

async fn handle_pty_ws(
    mut socket: axum::extract::ws::WebSocket,
    mut output_rx: broadcast::Receiver<Vec<u8>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: PtyMaster,
    event_bus: Arc<EventBus>,
    session_id: String,
) {
    loop {
        tokio::select! {
            result = output_rx.recv() => {
                match result {
                    Ok(bytes) => {
                        if socket.send(Message::Binary(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("ws client lagged by {} messages for session {}", n, session_id);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(ctrl) = serde_json::from_str::<ControlMessage>(&text) {
                            match ctrl {
                                ControlMessage::Resize { cols, rows } => {
                                    let _ = master.resize(cols, rows);
                                    event_bus.send(
                                        session_id.clone(),
                                        Event::Resized { cols, rows },
                                    );
                                }
                            }
                        } else {
                            let _ = writer.lock().unwrap().write_all(text.as_bytes());
                        }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        let _ = writer.lock().unwrap().write_all(&data);
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
