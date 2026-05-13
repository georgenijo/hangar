use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Result;
use regex::Regex;

use crate::events::{AgentEvent, TurnRole};
use crate::session::{SessionKind, SessionState};
use crate::util;

use super::{AgentDriver, OobMessage, PtyHandle, SpawnCfg, SpawnRequest, StateCtx};

// TODO: All regex statics below are UNVERIFIED — need real Codex CLI PTY output fixtures.
// Capture real output before shipping.

#[allow(dead_code)]
static CODEX_MODEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)model:\s*(\S+)").unwrap());

#[allow(dead_code)]
static CODEX_TOOL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:running|executing)\s+(\w+)\s*[\(\{]?(.*)").unwrap());

#[allow(dead_code)]
static CODEX_ERR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:error|failed)[:\s](.+)").unwrap());

#[allow(dead_code)]
static CODEX_TURN_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:done|finished|complete)").unwrap());

#[derive(Debug, PartialEq, Clone)]
#[allow(dead_code)]
enum ParserState {
    Idle,
    InThinkingBlock {
        turn_id: u64,
        char_count: u32,
    },
    InToolOutput {
        turn_id: u64,
        call_id: String,
        tool: String,
    },
    AwaitingInput,
}

pub struct CodexDriver {
    #[allow(dead_code)]
    turn_counter: u64,
    line_buffer: String,
    parser_state: ParserState,
    #[allow(dead_code)]
    last_model: Option<String>,
    #[allow(dead_code)]
    last_turn_start_ms: Option<u64>,
}

impl CodexDriver {
    pub fn new() -> Self {
        Self {
            turn_counter: 0,
            line_buffer: String::new(),
            parser_state: ParserState::Idle,
            last_model: None,
            last_turn_start_ms: None,
        }
    }

    #[allow(dead_code)]
    fn next_turn(&mut self) -> u64 {
        self.turn_counter += 1;
        self.turn_counter
    }

    fn process_line(&mut self, line: &str) -> Vec<AgentEvent> {
        let clean = util::strip_ansi(line);
        let clean = clean.trim();
        let events = Vec::new();

        // TODO: All patterns below are UNVERIFIED — need real Codex CLI PTY output fixtures.
        // Capture real output before shipping.

        // Model detection — TODO: verify Codex output format
        // Tool call detection — TODO: verify Codex tool output markers
        // Error detection — TODO: verify error line format
        // Turn markers — TODO: verify prompt/response boundaries

        let _ = clean;
        events
    }
}

impl Default for CodexDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentDriver for CodexDriver {
    fn kind(&self) -> &'static str {
        "codex"
    }

    fn spawn_cfg(&self, req: &SpawnRequest) -> Result<SpawnCfg> {
        // TODO: verify Codex CLI flags and invocation after real output capture
        let project_dir = match &req.kind {
            SessionKind::Codex { project_dir } => project_dir.clone(),
            _ => None,
        };

        let session_id = req.session_id.to_string();
        let mut command = vec!["codex".to_string(), "--full-auto".to_string()];

        if let Some(pd) = &project_dir {
            command.push("--cwd".to_string());
            command.push(pd.to_string_lossy().into_owned());
        }

        let mut env = req.env.clone();
        super::inherit_baseline_env(&mut env);
        env.insert("HANGAR_SESSION_ID".to_string(), session_id);

        Ok(SpawnCfg {
            command,
            env,
            cwd: req.cwd.clone(),
            temp_files: Vec::new(),
        })
    }

    fn on_bytes(&mut self, bytes: &[u8]) -> Vec<AgentEvent> {
        let s = String::from_utf8_lossy(bytes);
        self.line_buffer.push_str(&s);

        let mut events = Vec::new();
        while let Some(pos) = self.line_buffer.find('\n') {
            let line = self.line_buffer[..pos].to_string();
            self.line_buffer = self.line_buffer[pos + 1..].to_string();
            events.extend(self.process_line(&line));
        }
        events
    }

    fn on_oob(&mut self, _msg: OobMessage) -> Vec<AgentEvent> {
        // Codex has no HTTP hook mechanism
        Vec::new()
    }

    fn detect_state(&self, ctx: &StateCtx) -> Option<SessionState> {
        if self.parser_state == ParserState::AwaitingInput
            && ctx.current_state != SessionState::Awaiting
        {
            return Some(SessionState::Awaiting);
        }

        if let Some(AgentEvent::TurnFinished { .. }) = &ctx.last_event {
            let quiet_ms = util::now_ms() as i64 - ctx.last_bytes_ms;
            if quiet_ms > 3_000 && ctx.current_state != SessionState::Idle {
                return Some(SessionState::Idle);
            }
        }

        if let Some(AgentEvent::TurnStarted {
            role: TurnRole::Assistant,
            ..
        }) = &ctx.last_event
        {
            if ctx.current_state != SessionState::Streaming {
                return Some(SessionState::Streaming);
            }
        }

        let now = util::now_ms() as i64;
        if ctx.last_bytes_ms > 0
            && now - ctx.last_bytes_ms > 30_000
            && ctx.current_state == SessionState::Streaming
        {
            tracing::warn!("codex session may be hung: no bytes for >30s");
        }

        None
    }

    fn shutdown(&self, handle: &mut PtyHandle, _grace: Duration) -> Result<()> {
        handle.write_all(b"\x03")?;
        handle.write_all(b"exit\r")?;
        Ok(())
    }
}
