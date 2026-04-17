use std::time::Duration;

use anyhow::Result;

use crate::events::AgentEvent;
use crate::session::SessionState;

use super::{AgentDriver, OobMessage, PtyHandle, SpawnCfg, SpawnRequest, StateCtx};

pub struct RawBytesDriver;

impl RawBytesDriver {
    pub fn new() -> Self {
        Self
    }
}

impl AgentDriver for RawBytesDriver {
    fn kind(&self) -> &'static str {
        "raw_bytes"
    }

    fn spawn_cfg(&self, req: &SpawnRequest) -> Result<SpawnCfg> {
        Ok(SpawnCfg {
            command: vec!["cat".to_string()],
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

    fn shutdown(&self, _handle: &mut PtyHandle, _grace: Duration) -> Result<()> {
        Ok(())
    }
}

impl Default for RawBytesDriver {
    fn default() -> Self {
        Self::new()
    }
}
