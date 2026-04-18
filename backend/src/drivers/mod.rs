pub mod claude_code;
/// Claude TUI status-line scraper. Parses "CTX 3% 30k $0.11 | ... | claude-opus-4-7[1m] | ..."
/// shared across drivers (both claude_code sessions and shells that host a claude process).
pub mod status_scraper {
    use crate::events::AgentEvent;
    use crate::util;
    use regex::Regex;
    use std::sync::LazyLock;

    static STATUS_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"CTX\s*(\d+)%\s*(\d+)k\s*\$([\d.]+)").unwrap());
    static STATUS_MODEL_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\|\s+(claude-[A-Za-z0-9\-]+(?:\[\d+m?\])?)\s+\|").unwrap());

    // Claude TUI cursor-positions the bullet glyph away from the tool name in the raw
    // byte stream, so anchor on known tool-name tokens instead.
    static TOOL_LINE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"\b(Bash|BashOutput|Glob|Grep|Read|Write|Edit|MultiEdit|LS|Task|WebFetch|WebSearch|NotebookEdit|KillBash|TodoWrite|SlashCommand|ExitPlanMode)\(([^)]{0,200})",
        )
        .unwrap()
    });

    #[derive(Default)]
    pub struct ScraperState {
        pub last_dollars: Option<f64>,
        pub last_pct: Option<u32>,
        pub last_model: Option<String>,
        pub line_buf: String,
        pub status_buf: String,
        pub synth_turn: u64,
        pub synth_call: u64,
    }

    /// Status-only scrape: CTX% / tokens / cost / model. No tool detection.
    /// Use this from drivers that already have a hook-based tool stream (claude_code).
    pub fn scrape_status(chunk: &str, state: &mut ScraperState) -> Vec<AgentEvent> {
        let clean = util::strip_ansi(chunk);
        state.status_buf.push_str(&clean);
        // Keep rolling window bounded so regex scanning stays cheap.
        // Slice at a char boundary — status_buf is UTF-8 and contains non-ASCII glyphs.
        if state.status_buf.len() > 4096 {
            let mut cut = state.status_buf.len() - 2048;
            while cut < state.status_buf.len() && !state.status_buf.is_char_boundary(cut) {
                cut += 1;
            }
            state.status_buf = state.status_buf[cut..].to_string();
        }
        let mut events = Vec::new();
        // Scan the rolling buffer so patterns split across chunks still match.
        // Use last-match semantics: the freshest status line dominates.
        if let Some(cap) = STATUS_RE.captures_iter(&state.status_buf).last() {
            let pct: u32 = cap[1].parse().unwrap_or(0);
            let tokens_k: u64 = cap[2].parse().unwrap_or(0);
            let dollars: f64 = cap[3].parse().unwrap_or(0.0);
            let changed = state.last_dollars != Some(dollars) || state.last_pct != Some(pct);
            if changed {
                state.last_dollars = Some(dollars);
                state.last_pct = Some(pct);
                events.push(AgentEvent::ContextWindowSizeChanged {
                    pct_used: pct as f32 / 100.0,
                    tokens: tokens_k * 1000,
                });
                events.push(AgentEvent::CostUpdated { dollars });
            }
        }
        if let Some(cap) = STATUS_MODEL_RE.captures_iter(&state.status_buf).last() {
            let model = cap[1].to_string();
            if state.last_model.as_deref() != Some(&model) {
                state.last_model = Some(model.clone());
                events.push(AgentEvent::ModelChanged { model });
            }
        }
        events
    }

    /// Full scrape: status + line-level tool-call detection for hook-less sessions.
    pub fn scrape_all(chunk: &str, state: &mut ScraperState) -> Vec<AgentEvent> {
        let mut events = scrape_status(chunk, state);
        let clean = util::strip_ansi(chunk);

        // Line-buffered tool-call scrape
        state.line_buf.push_str(&clean);
        while let Some(pos) = state.line_buf.find('\n') {
            let line = state.line_buf[..pos].to_string();
            state.line_buf = state.line_buf[pos + 1..].to_string();
            if let Some(cap) = TOOL_LINE_RE.captures(&line) {
                let tool = cap[1].to_string();
                let args_preview = cap[2].trim().to_string();
                state.synth_call += 1;
                let call_id = format!("scraped-{}", state.synth_call);
                // Synthetic turn id: reuse latest or bump
                if state.synth_turn == 0 {
                    state.synth_turn = 1;
                }
                events.push(AgentEvent::ToolCallStarted {
                    turn_id: state.synth_turn,
                    call_id: call_id.clone(),
                    tool,
                    args_preview,
                });
                // Emit an immediate finished entry (we don't observe completion separately)
                events.push(AgentEvent::ToolCallFinished {
                    turn_id: state.synth_turn,
                    call_id,
                    ok: true,
                    result_preview: String::new(),
                });
            }
        }

        events
    }
}

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
