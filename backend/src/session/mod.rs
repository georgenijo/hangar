use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    Booting,
    Idle,
    Streaming,
    Awaiting,
    Error,
    Exited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionKind {
    Shell { command: Vec<String> },
    ClaudeCode {
        config_override: Option<PathBuf>,
        project_dir: Option<PathBuf>,
    },
    RawBytes { command: Vec<String> },
}
