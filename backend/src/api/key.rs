use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::resolve_session;
use crate::AppState;

#[derive(Deserialize)]
pub struct KeyRequest {
    pub key: String,
}

fn key_bytes(name: &str) -> Option<&'static [u8]> {
    match name {
        "Enter" => Some(b"\r"),
        "Tab" => Some(b"\t"),
        "Ctrl-c" => Some(b"\x03"),
        "Ctrl-d" => Some(b"\x04"),
        "Ctrl-z" => Some(b"\x1a"),
        "Ctrl-l" => Some(b"\x0c"),
        "Ctrl-a" => Some(b"\x01"),
        "Ctrl-e" => Some(b"\x05"),
        "Ctrl-u" => Some(b"\x15"),
        "Ctrl-w" => Some(b"\x17"),
        "Escape" => Some(b"\x1b"),
        "Up" => Some(b"\x1b[A"),
        "Down" => Some(b"\x1b[B"),
        "Right" => Some(b"\x1b[C"),
        "Left" => Some(b"\x1b[D"),
        "Backspace" => Some(b"\x7f"),
        "Delete" => Some(b"\x1b[3~"),
        "Home" => Some(b"\x1b[H"),
        "End" => Some(b"\x1b[F"),
        _ => None,
    }
}

pub async fn send_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<KeyRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let key = body.key.trim();

    let bytes = key_bytes(key).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "unknown_key", "key": key})),
        )
    })?;

    let session = resolve_session(&state, &id)
        .await
        .map_err(|s| (s, Json(serde_json::json!({}))))?;
    let ulid = session.id.to_string();

    let sessions = state.sessions.read().unwrap();
    let active = sessions
        .get(&ulid)
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({}))))?;

    active
        .writer
        .lock()
        .unwrap()
        .write_all(bytes)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({})),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_bytes_known() {
        assert_eq!(key_bytes("Enter"), Some(b"\r" as &[u8]));
        assert_eq!(key_bytes("Tab"), Some(b"\t" as &[u8]));
        assert_eq!(key_bytes("Ctrl-c"), Some(b"\x03" as &[u8]));
        assert_eq!(key_bytes("Ctrl-d"), Some(b"\x04" as &[u8]));
        assert_eq!(key_bytes("Escape"), Some(b"\x1b" as &[u8]));
        assert_eq!(key_bytes("Up"), Some(b"\x1b[A" as &[u8]));
        assert_eq!(key_bytes("Down"), Some(b"\x1b[B" as &[u8]));
        assert_eq!(key_bytes("Left"), Some(b"\x1b[D" as &[u8]));
        assert_eq!(key_bytes("Right"), Some(b"\x1b[C" as &[u8]));
        assert_eq!(key_bytes("Backspace"), Some(b"\x7f" as &[u8]));
    }

    #[test]
    fn test_key_bytes_unknown() {
        assert_eq!(key_bytes("F1"), None);
        assert_eq!(key_bytes("UnknownKey"), None);
        assert_eq!(key_bytes(""), None);
    }
}
