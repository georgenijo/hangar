use std::time::Duration;

use anyhow::Result;

use crate::events::AgentEvent;
use crate::session::SessionState;

use super::{AgentDriver, OobMessage, PtyHandle, SpawnCfg, SpawnRequest, StateCtx};

pub struct ShellDriver;

impl ShellDriver {
    pub fn new() -> Self {
        Self
    }
}

impl AgentDriver for ShellDriver {
    fn kind(&self) -> &'static str {
        "shell"
    }

    fn spawn_cfg(&self, req: &SpawnRequest) -> Result<SpawnCfg> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
        Ok(SpawnCfg {
            command: vec![shell, "-l".to_string()],
            env: req.env.clone(),
            cwd: req.cwd.clone(),
            temp_files: Vec::new(),
        })
    }

    fn on_bytes(&mut self, _bytes: &[u8]) -> Vec<AgentEvent> {
        Vec::new()
    }

    fn on_oob(&mut self, _msg: OobMessage) -> Vec<AgentEvent> {
        Vec::new()
    }

    fn detect_state(&self, _ctx: &StateCtx) -> Option<SessionState> {
        None
    }

    fn shutdown(&self, handle: &mut PtyHandle, _grace: Duration) -> Result<()> {
        handle.write_all(b"exit\r")?;
        Ok(())
    }
}

impl Default for ShellDriver {
    fn default() -> Self {
        Self::new()
    }
}
