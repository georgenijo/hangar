use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use portable_pty::MasterPty;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use tokio::sync::broadcast;
use ulid::Ulid;

pub type SessionId = Ulid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub id: String,
    pub slug: String,
    pub kind: SessionKind,
    pub state: SessionState,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    pub agent_meta: Option<AgentMeta>,
    pub labels: BTreeMap<String, String>,
    pub node_id: String,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub exit: Option<ExitInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum SessionKind {
    Shell {
        command: Vec<String>,
    },
    ClaudeCode {
        config_override: Option<PathBuf>,
        project_dir: Option<PathBuf>,
    },
    RawBytes {
        command: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum SessionState {
    Booting,
    Idle,
    Streaming,
    Awaiting,
    Error,
    Exited,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentMeta {
    pub agent_type: String,
    pub model: Option<String>,
    pub config_path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExitInfo {
    pub code: Option<i32>,
    pub signal: Option<String>,
    pub exited_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub slug: String,
    pub kind: SessionKind,
    pub cwd: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
    pub labels: Option<BTreeMap<String, String>>,
}

#[derive(Deserialize)]
pub struct ResizeRequest {
    pub cols: u16,
    pub rows: u16,
}

pub struct SessionHandle {
    pub session: Mutex<Session>,
    pub pty_master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    pub pty_writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub output_tx: broadcast::Sender<Vec<u8>>,
    pub child: Arc<Mutex<Box<dyn portable_pty::Child + Send>>>,
}

impl SessionHandle {
    pub fn try_wait(&self) -> Option<portable_pty::ExitStatus> {
        self.child.lock().unwrap().try_wait().ok().flatten()
    }
}

pub type SessionStore = Arc<DashMap<String, Arc<SessionHandle>>>;

static SLUG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]{0,31}$").unwrap());

pub fn validate_slug(slug: &str) -> bool {
    SLUG_RE.is_match(slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_slugs() {
        assert!(validate_slug("a"));
        assert!(validate_slug("my-shell"));
        assert!(validate_slug("shell-01"));
        assert!(validate_slug("abcdefghijklmnopqrstuvwxyz012345"));
    }

    #[test]
    fn test_invalid_slugs() {
        assert!(!validate_slug(""));
        assert!(!validate_slug("A"));
        assert!(!validate_slug("1abc"));
        assert!(!validate_slug("-abc"));
        assert!(!validate_slug("ab_cd"));
        assert!(!validate_slug(&"a".repeat(33)));
        assert!(!validate_slug("ab cd"));
        assert!(!validate_slug("ab.cd"));
    }

    #[test]
    fn test_session_kind_serde() {
        let kind = SessionKind::Shell {
            command: vec!["/bin/bash".into(), "-l".into()],
        };
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, r#"{"Shell":{"command":["/bin/bash","-l"]}}"#);
        let roundtrip: SessionKind = serde_json::from_str(&json).unwrap();
        match roundtrip {
            SessionKind::Shell { command } => {
                assert_eq!(command, vec!["/bin/bash", "-l"]);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_session_state_serde() {
        let json = serde_json::to_string(&SessionState::Booting).unwrap();
        assert_eq!(json, r#""Booting""#);
    }

    #[test]
    fn test_claude_code_kind_serde() {
        let kind = SessionKind::ClaudeCode {
            config_override: None,
            project_dir: None,
        };
        let json = serde_json::to_string(&kind).unwrap();
        let roundtrip: SessionKind = serde_json::from_str(&json).unwrap();
        match roundtrip {
            SessionKind::ClaudeCode { .. } => {}
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_raw_bytes_kind_serde() {
        let kind = SessionKind::RawBytes {
            command: vec!["vim".into()],
        };
        let json = serde_json::to_string(&kind).unwrap();
        let roundtrip: SessionKind = serde_json::from_str(&json).unwrap();
        match roundtrip {
            SessionKind::RawBytes { command } => {
                assert_eq!(command, vec!["vim"]);
            }
            _ => panic!("wrong variant"),
        }
    }
}
