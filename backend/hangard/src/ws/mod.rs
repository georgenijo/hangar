use std::io::Write;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;

use crate::api::{find_session, AppState};
use crate::session::SessionHandle;

pub fn router() -> Router<AppState> {
    Router::new().route("/ws/v1/sessions/{id}/pty", get(ws_handler))
}

async fn ws_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let handle = find_session(&state.sessions, &id);
    match handle {
        Some(h) => ws.on_upgrade(move |socket| handle_pty_ws(socket, h)),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn handle_pty_ws(socket: WebSocket, handle: Arc<SessionHandle>) {
    let (mut sender, mut receiver) = socket.split();
    let mut broadcast_rx: broadcast::Receiver<Vec<u8>> = handle.output_tx.subscribe();

    let pty_writer = handle.pty_writer.clone();
    let pty_master = handle.pty_master.clone();

    let reader_task = tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(bytes) => {
                    if sender.send(Message::Binary(bytes.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("ws client lagged, dropped {n} messages");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let writer_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Binary(bytes)) => {
                    let w = pty_writer.clone();
                    let bytes = bytes.to_vec();
                    tokio::task::spawn_blocking(move || {
                        if let Ok(mut writer) = w.lock() {
                            let _ = writer.write_all(&bytes);
                        }
                    })
                    .await
                    .ok();
                }
                Ok(Message::Text(text)) => {
                    if let Ok(resize) = serde_json::from_str::<WsResizeMsg>(&text) {
                        if resize.r#type == "resize" {
                            if let Ok(master) = pty_master.lock() {
                                let _ = crate::pty::resize_pty(&**master, resize.cols, resize.rows);
                            }
                        }
                    } else {
                        let w = pty_writer.clone();
                        let bytes = text.as_bytes().to_vec();
                        tokio::task::spawn_blocking(move || {
                            if let Ok(mut writer) = w.lock() {
                                let _ = writer.write_all(&bytes);
                            }
                        })
                        .await
                        .ok();
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = reader_task => {},
        _ = writer_task => {},
    }
}

#[derive(Deserialize)]
struct WsResizeMsg {
    r#type: String,
    cols: u16,
    rows: u16,
}
