pub mod claude_code;
pub mod codex;
pub mod raw_bytes;
pub mod shell;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use serde_json::Value;

use crate::events::AgentEvent;
use crate::session::{SessionId, SessionKind, SessionState};

pub struct SpawnRequest {
    pub session_id: SessionId,
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,
    pub kind: SessionKind,
    pub hmac_key: Vec<u8>,
}

pub struct SpawnCfg {
    pub command: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: PathBuf,
    pub temp_files: Vec<PathBuf>,
}

pub struct PtyHandle {
    writer: Box<dyn std::io::Write + Send>,
}

impl PtyHandle {
    pub fn new(writer: Box<dyn std::io::Write + Send>) -> Self {
        Self { writer }
    }

    pub fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(data)
    }
}

pub struct OobMessage {
    pub hook: String,
    pub ts: String,
    pub payload: Value,
}

pub struct StateCtx {
    pub current_state: SessionState,
    pub last_activity_ms: i64,
    pub last_event: Option<AgentEvent>,
    pub last_bytes_ms: i64,
    pub event_timestamps: Vec<i64>,
}

pub trait AgentDriver: Send + Sync + 'static {
    fn kind(&self) -> &'static str;
    fn spawn_cfg(&self, req: &SpawnRequest) -> Result<SpawnCfg>;
    fn on_bytes(&mut self, bytes: &[u8]) -> Vec<AgentEvent>;
    fn on_oob(&mut self, msg: OobMessage) -> Vec<AgentEvent>;
    fn detect_state(&self, ctx: &StateCtx) -> Option<SessionState>;
    fn prompt(&self, handle: &mut PtyHandle, text: &str) -> Result<()> {
        handle.write_all(text.as_bytes())?;
        handle.write_all(b"\r")?;
        Ok(())
    }
    fn shutdown(&self, handle: &mut PtyHandle, grace: Duration) -> Result<()>;
}

pub struct DriverRegistry {
    drivers: HashMap<String, Box<dyn Fn() -> Box<dyn AgentDriver> + Send + Sync>>,
}

impl DriverRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            drivers: HashMap::new(),
        };
        reg.register("shell", || Box::new(shell::ShellDriver::new()));
        reg.register("claude_code", || {
            Box::new(claude_code::ClaudeCodeDriver::new())
        });
        reg.register("raw_bytes", || Box::new(raw_bytes::RawBytesDriver::new()));
        reg.register("codex", || Box::new(codex::CodexDriver::new()));
        reg
    }

    pub fn register<F>(&mut self, kind: &str, factory: F)
    where
        F: Fn() -> Box<dyn AgentDriver> + Send + Sync + 'static,
    {
        self.drivers.insert(kind.to_string(), Box::new(factory));
    }

    pub fn create(&self, kind: &str) -> Option<Box<dyn AgentDriver>> {
        self.drivers.get(kind).map(|f| f())
    }
}

impl Default for DriverRegistry {
    fn default() -> Self {
        Self::new()
    }
}
