// TODO: Add fixture tests after capturing real Codex PTY output

use hangard::{
    drivers::{codex::CodexDriver, AgentDriver, OobMessage, PtyHandle, StateCtx},
    session::{SessionId, SessionKind, SessionState},
};
use std::collections::HashMap;

fn make_oob(hook: &str) -> OobMessage {
    OobMessage {
        hook: hook.to_string(),
        ts: "2024-01-01T00:00:00Z".to_string(),
        payload: serde_json::json!({}),
    }
}

#[test]
fn test_codex_on_bytes_empty() {
    let mut d = CodexDriver::new();
    let events = d.on_bytes(b"");
    assert!(events.is_empty());
}

#[test]
fn test_codex_on_oob_returns_empty() {
    let mut d = CodexDriver::new();
    let events = d.on_oob(make_oob("AnyHook"));
    assert!(events.is_empty(), "Codex has no OOB hook support");
}

#[test]
fn test_codex_spawn_cfg_command() {
    use hangard::drivers::SpawnRequest;

    let d = CodexDriver::new();
    let req = SpawnRequest {
        session_id: SessionId::new(),
        cwd: std::path::PathBuf::from("/tmp"),
        env: HashMap::new(),
        kind: SessionKind::Codex { project_dir: None },
        hmac_key: vec![],
    };
    let cfg = d.spawn_cfg(&req).unwrap();
    assert!(
        cfg.command.first().map(|s| s.as_str()) == Some("codex"),
        "command should start with 'codex'"
    );
    assert!(
        cfg.env.contains_key("HANGAR_SESSION_ID"),
        "env should contain HANGAR_SESSION_ID"
    );
}

#[test]
fn test_codex_detect_state_hung_detection() {
    let d = CodexDriver::new();
    // last_bytes_ms set to > 30s ago — hung detection logs a warning but returns None
    let old_ts = hangard::util::now_ms() as i64 - 35_000;
    let ctx = StateCtx {
        current_state: SessionState::Streaming,
        last_activity_ms: 0,
        last_event: None,
        last_bytes_ms: old_ts,
        event_timestamps: vec![],
    };
    // Hung detection is log-only for now, so detect_state returns None
    let result = d.detect_state(&ctx);
    assert!(
        result.is_none(),
        "hung detection is log-only, should return None"
    );
}

#[test]
fn test_codex_kind() {
    let d = CodexDriver::new();
    assert_eq!(d.kind(), "codex");
}

#[test]
fn test_codex_shutdown() {
    let d = CodexDriver::new();
    let buf: Vec<u8> = Vec::new();
    let cursor = std::io::Cursor::new(buf);
    let mut handle = PtyHandle::new(Box::new(cursor));
    let result = d.shutdown(&mut handle, std::time::Duration::from_secs(5));
    assert!(result.is_ok(), "shutdown should not error");
}
