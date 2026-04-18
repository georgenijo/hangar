use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Result;
use regex::Regex;

use crate::events::{AgentEvent, TurnRole};
use crate::session::{SessionKind, SessionState};
use crate::util;

use super::{AgentDriver, OobMessage, PtyHandle, SpawnCfg, SpawnRequest, StateCtx};

static MODEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:using\s+)?model:\s*(\S+)").unwrap());
static CTX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)context\s+window[:\s]*(\d+)%.*?(\d[\d,]*)\s+tokens?").unwrap()
});
static PERM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:allow|deny|permission)[^\n]*?(\w+)[^\n]*\?").unwrap());
static THINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^thinking\.{0,3}$|^<thinking>").unwrap());
static TOOL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[⏺●•]\s+(\w+)\s*[\(\{]?(.*)").unwrap());
static PROMPT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[❯>]\s").unwrap());
static ERR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:error|api error|rate limit)[:\s](.+)").unwrap());
// UNVERIFIED — needs fixture capture from real Claude Code session
static COMPACT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)compact(?:ing|ed|ion)").unwrap());
// UNVERIFIED — needs fixture capture
static SUBAGENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:spawning|starting)\s+(?:sub-?agent|task)").unwrap());
// UNVERIFIED — needs fixture capture
static THINK_BUDGET_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)thinking\s+budget\s+(?:exceeded|exhausted|limit)").unwrap());

#[derive(Debug, PartialEq, Clone)]
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

pub struct ClaudeCodeDriver {
    hooks_active: bool,
    turn_counter: u64,
    line_buffer: String,
    parser_state: ParserState,
    last_model: Option<String>,
    last_tool_call: Option<String>,
    last_turn_start_ms: Option<u64>,
}

impl ClaudeCodeDriver {
    pub fn new() -> Self {
        Self {
            hooks_active: false,
            turn_counter: 0,
            line_buffer: String::new(),
            parser_state: ParserState::Idle,
            last_model: None,
            last_tool_call: None,
            last_turn_start_ms: None,
        }
    }

    fn next_turn(&mut self) -> u64 {
        self.turn_counter += 1;
        self.turn_counter
    }

    fn process_line(&mut self, line: &str) -> Vec<AgentEvent> {
        let clean = util::strip_ansi(line);
        let clean = clean.trim();
        let mut events = Vec::new();

        // Model detection
        if let Some(cap) = MODEL_RE.captures(clean) {
            let model = cap[1].to_string();
            self.last_model = Some(model.clone());
            events.push(AgentEvent::ModelChanged { model });
            return events;
        }

        // Context window: "Context window: 50% (100,000 tokens)"
        if let Some(cap) = CTX_RE.captures(clean) {
            let pct: f32 = cap[1].parse::<f32>().unwrap_or(0.0) / 100.0;
            let tokens_str = cap[2].replace(',', "");
            let tokens: u64 = tokens_str.parse().unwrap_or(0);
            events.push(AgentEvent::ContextWindowSizeChanged {
                pct_used: pct,
                tokens,
            });
            return events;
        }

        // Permission prompt: CC shows "Allow [tool]?" or "Do you want to allow..."
        if let Some(cap) = PERM_RE.captures(clean) {
            let tool = cap[1].to_string();
            let prompt = clean.to_string();
            self.parser_state = ParserState::AwaitingInput;
            events.push(AgentEvent::AwaitingPermission { tool, prompt });
            return events;
        }

        // Thinking block start: "Thinking..." or extended thinking markers
        if THINK_RE.is_match(clean) {
            let turn_id = self.turn_counter;
            self.parser_state = ParserState::InThinkingBlock {
                turn_id,
                char_count: 0,
            };
            return events;
        }

        // Thinking block end
        if clean == "</thinking>" {
            if let ParserState::InThinkingBlock {
                turn_id,
                char_count,
            } = self.parser_state.clone()
            {
                events.push(AgentEvent::ThinkingBlock {
                    turn_id,
                    len_chars: char_count,
                });
                self.parser_state = ParserState::Idle;
            }
            return events;
        }

        // Accumulate thinking block chars
        if let ParserState::InThinkingBlock {
            turn_id,
            ref mut char_count,
        } = self.parser_state
        {
            *char_count += clean.chars().count() as u32;
            let _ = turn_id;
            return events;
        }

        // Tool call: CC uses "⏺ ToolName(..." pattern
        if let Some(cap) = TOOL_RE.captures(clean) {
            let tool = cap[1].to_string();
            let args_preview = cap[2].chars().take(100).collect::<String>();
            let turn_id = self.turn_counter;
            let call_id = format!("{}-{}", turn_id, &tool);
            self.last_tool_call = Some(tool.clone());
            self.parser_state = ParserState::InToolOutput {
                turn_id,
                call_id: call_id.clone(),
                tool: tool.clone(),
            };
            events.push(AgentEvent::ToolCallStarted {
                turn_id,
                call_id,
                tool,
                args_preview,
            });
            return events;
        }

        // Tool output end — blank line after tool output signals finish
        if clean.is_empty() {
            if let ParserState::InToolOutput {
                turn_id,
                call_id,
                tool: _,
            } = self.parser_state.clone()
            {
                events.push(AgentEvent::ToolCallFinished {
                    turn_id,
                    call_id,
                    ok: true,
                    result_preview: String::new(),
                });
                self.parser_state = ParserState::Idle;
            }
            return events;
        }

        // Turn start: prompt markers ❯ or > at start of line suggest user input prompt
        if PROMPT_RE.is_match(clean) {
            let turn_id = self.next_turn();
            self.last_turn_start_ms = Some(util::now_ms());
            events.push(AgentEvent::TurnStarted {
                turn_id,
                role: TurnRole::User,
                content_start: None,
            });
            return events;
        }

        // Error patterns
        if let Some(cap) = ERR_RE.captures(clean) {
            events.push(AgentEvent::Error {
                message: cap[1].to_string(),
            });
            return events;
        }

        // UNVERIFIED — compaction line format needs fixture capture
        if COMPACT_RE.is_match(clean) {
            events.push(AgentEvent::ContextWindowSizeChanged {
                pct_used: 0.0,
                tokens: 0,
            });
            return events;
        }

        // UNVERIFIED — subagent spawn format needs fixture capture
        if SUBAGENT_RE.is_match(clean) {
            let args_preview = clean.chars().take(100).collect::<String>();
            let turn_id = self.turn_counter;
            let call_id = format!("{}-subagent", turn_id);
            events.push(AgentEvent::ToolCallStarted {
                turn_id,
                call_id,
                tool: "subagent".to_string(),
                args_preview,
            });
            return events;
        }

        // UNVERIFIED — thinking budget exhaustion format needs fixture capture
        if THINK_BUDGET_RE.is_match(clean) {
            events.push(AgentEvent::Error {
                message: clean.to_string(),
            });
            return events;
        }

        events
    }
}

impl Default for ClaudeCodeDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentDriver for ClaudeCodeDriver {
    fn kind(&self) -> &'static str {
        "claude_code"
    }

    fn spawn_cfg(&self, req: &SpawnRequest) -> Result<SpawnCfg> {
        let (config_override, project_dir) = match &req.kind {
            SessionKind::ClaudeCode {
                config_override,
                project_dir,
            } => (config_override.clone(), project_dir.clone()),
            _ => (None, None),
        };

        let port: u16 = std::env::var("HANGAR_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        let session_id = req.session_id.to_string();
        let hook_url = format!("http://127.0.0.1:{port}/_cc_hook/{session_id}");

        let temp_dir = tempfile::TempDir::new()?;
        let settings_path = temp_dir.path().join("hangar-settings.json");

        let hooks_config = if let Some(override_path) = &config_override {
            let existing =
                std::fs::read_to_string(override_path).unwrap_or_else(|_| "{}".to_string());
            let mut v: serde_json::Value =
                serde_json::from_str(&existing).unwrap_or(serde_json::json!({}));
            v["hooks"] = build_hooks_config(&hook_url);
            v
        } else {
            serde_json::json!({ "hooks": build_hooks_config(&hook_url) })
        };

        std::fs::write(&settings_path, serde_json::to_string_pretty(&hooks_config)?)?;

        let settings_path_str = settings_path.to_string_lossy().into_owned();
        let temp_dir_path = temp_dir.path().to_path_buf();
        std::mem::forget(temp_dir);

        let command = vec![
            "claude".to_string(),
            "--dangerously-skip-permissions".to_string(),
            "--settings".to_string(),
            settings_path_str,
        ];

        let mut env = req.env.clone();
        env.insert("HANGAR_SESSION_ID".to_string(), session_id);
        env.insert("HANGAR_HMAC_KEY".to_string(), hex::encode(&req.hmac_key));

        let cwd = project_dir.unwrap_or_else(|| req.cwd.clone());

        Ok(SpawnCfg {
            command,
            env,
            cwd,
            temp_files: vec![temp_dir_path],
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

    fn on_oob(&mut self, msg: OobMessage) -> Vec<AgentEvent> {
        self.hooks_active = true;
        let mut events = Vec::new();

        match msg.hook.as_str() {
            "SessionStart" => {
                let turn_id = self.next_turn();
                self.last_turn_start_ms = Some(util::now_ms());
                events.push(AgentEvent::TurnStarted {
                    turn_id,
                    role: TurnRole::System,
                    content_start: None,
                });
                if let Some(model) = msg.payload.get("model").and_then(|v| v.as_str()) {
                    self.last_model = Some(model.to_string());
                    events.push(AgentEvent::ModelChanged {
                        model: model.to_string(),
                    });
                }
            }

            "UserPromptSubmit" => {
                let turn_id = self.next_turn();
                self.last_turn_start_ms = Some(util::now_ms());
                let content_start = msg
                    .payload
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .map(|s| s.chars().take(100).collect());
                events.push(AgentEvent::TurnStarted {
                    turn_id,
                    role: TurnRole::User,
                    content_start,
                });
            }

            "PreToolUse" => {
                let turn_id = self.turn_counter;
                let tool = msg
                    .payload
                    .get("tool_name")
                    .or_else(|| msg.payload.get("tool"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let call_id = format!("{}-{}", turn_id, &tool);
                let args_preview = msg
                    .payload
                    .get("tool_input")
                    .map(|v| v.to_string().chars().take(100).collect::<String>())
                    .unwrap_or_default();
                self.last_tool_call = Some(tool.clone());
                events.push(AgentEvent::ToolCallStarted {
                    turn_id,
                    call_id,
                    tool,
                    args_preview,
                });
            }

            "PostToolUse" => {
                let turn_id = self.turn_counter;
                let tool = msg
                    .payload
                    .get("tool_name")
                    .or_else(|| msg.payload.get("tool"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let call_id = format!("{}-{}", turn_id, &tool);
                let ok = msg.payload.get("error").is_none();
                let result_preview = msg
                    .payload
                    .get("tool_result")
                    .or_else(|| msg.payload.get("result"))
                    .map(|v| v.to_string().chars().take(100).collect::<String>())
                    .unwrap_or_default();
                events.push(AgentEvent::ToolCallFinished {
                    turn_id,
                    call_id,
                    ok,
                    result_preview,
                });
            }

            "Notification" => {
                // Only emit AwaitingPermission for actual permission notifications
                let is_permission = msg
                    .payload
                    .get("type")
                    .and_then(|v| v.as_str())
                    .map(|t| t == "permission")
                    .unwrap_or(false)
                    || msg
                        .payload
                        .get("title")
                        .and_then(|v| v.as_str())
                        .map(|t| {
                            let lower = t.to_lowercase();
                            lower.contains("permission") || lower.contains("allow")
                        })
                        .unwrap_or(false);

                if is_permission {
                    let tool = msg
                        .payload
                        .get("tool")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let prompt = msg
                        .payload
                        .get("message")
                        .or_else(|| msg.payload.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    self.parser_state = ParserState::AwaitingInput;
                    events.push(AgentEvent::AwaitingPermission { tool, prompt });
                } else if msg
                    .payload
                    .get("type")
                    .and_then(|v| v.as_str())
                    .map(|t| t == "error")
                    .unwrap_or(false)
                {
                    let message = msg
                        .payload
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error")
                        .to_string();
                    events.push(AgentEvent::Error { message });
                }
            }

            "Stop" => {
                let turn_id = self.turn_counter;
                let tokens_used = msg
                    .payload
                    .get("usage")
                    .and_then(|u| u.get("output_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let duration_ms = self
                    .last_turn_start_ms
                    .map(|start| (util::now_ms() - start) as u32)
                    .unwrap_or(0);
                self.last_turn_start_ms = None;
                events.push(AgentEvent::TurnFinished {
                    turn_id,
                    tokens_used,
                    duration_ms,
                });
            }

            _ => {
                tracing::warn!("unknown CC hook: {}", msg.hook);
            }
        }

        events
    }

    fn detect_state(&self, ctx: &StateCtx) -> Option<SessionState> {
        if self.parser_state == ParserState::AwaitingInput
            && ctx.current_state != SessionState::Awaiting
        {
            return Some(SessionState::Awaiting);
        }

        if let Some(AgentEvent::TurnFinished { .. }) = &ctx.last_event {
            if ctx.current_state != SessionState::Idle {
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

        None
    }

    fn shutdown(&self, handle: &mut PtyHandle, _grace: Duration) -> Result<()> {
        handle.write_all(b"/exit\r")?;
        Ok(())
    }
}

fn build_hooks_config(hook_base_url: &str) -> serde_json::Value {
    let make_entry = |hook_name: &str| {
        let url = format!("{}/{}", hook_base_url, hook_name);
        serde_json::json!([{
            "matcher": "",
            "hooks": [{
                "type": "command",
                "command": format!(
                    "curl -s -X POST -H 'Content-Type: application/json' --data @- '{}' >/dev/null 2>&1 || true",
                    url
                )
            }]
        }])
    };
    serde_json::json!({
        "PreToolUse": make_entry("PreToolUse"),
        "PostToolUse": make_entry("PostToolUse"),
        "Notification": make_entry("Notification"),
        "Stop": make_entry("Stop"),
        "SessionStart": make_entry("SessionStart"),
        "UserPromptSubmit": make_entry("UserPromptSubmit"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oob(hook: &str, payload: serde_json::Value) -> OobMessage {
        OobMessage {
            hook: hook.to_string(),
            ts: "2024-01-01T00:00:00Z".to_string(),
            payload,
        }
    }

    #[test]
    fn test_oob_session_start() {
        let mut d = ClaudeCodeDriver::new();
        let events = d.on_oob(oob(
            "SessionStart",
            serde_json::json!({"model": "claude-opus-4"}),
        ));
        assert!(events.iter().any(|e| matches!(
            e,
            AgentEvent::TurnStarted {
                role: TurnRole::System,
                ..
            }
        )));
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::ModelChanged { model } if model == "claude-opus-4")));
    }

    #[test]
    fn test_oob_pre_tool_use() {
        let mut d = ClaudeCodeDriver::new();
        let events = d.on_oob(oob(
            "PreToolUse",
            serde_json::json!({"tool_name": "Bash", "tool_input": {"command": "ls"}}),
        ));
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::ToolCallStarted { tool, .. } if tool == "Bash")));
    }

    #[test]
    fn test_oob_post_tool_use() {
        let mut d = ClaudeCodeDriver::new();
        let events = d.on_oob(oob(
            "PostToolUse",
            serde_json::json!({"tool_name": "Bash", "tool_result": "ok"}),
        ));
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::ToolCallFinished { ok: true, .. })));
    }

    #[test]
    fn test_oob_post_tool_use_error() {
        let mut d = ClaudeCodeDriver::new();
        let events = d.on_oob(oob(
            "PostToolUse",
            serde_json::json!({"tool_name": "Bash", "error": "permission denied"}),
        ));
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::ToolCallFinished { ok: false, .. })));
    }

    #[test]
    fn test_oob_notification_permission() {
        let mut d = ClaudeCodeDriver::new();
        let events = d.on_oob(oob(
            "Notification",
            serde_json::json!({"type": "permission", "tool": "Read", "message": "Allow?"}),
        ));
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::AwaitingPermission { .. })));
    }

    #[test]
    fn test_oob_notification_other() {
        let mut d = ClaudeCodeDriver::new();
        let events = d.on_oob(oob(
            "Notification",
            serde_json::json!({"type": "info", "message": "Starting up"}),
        ));
        assert!(events.is_empty());
    }

    #[test]
    fn test_oob_stop() {
        let mut d = ClaudeCodeDriver::new();
        let events = d.on_oob(oob(
            "Stop",
            serde_json::json!({"usage": {"output_tokens": 42}}),
        ));
        assert!(events.iter().any(|e| matches!(
            e,
            AgentEvent::TurnFinished {
                tokens_used: 42,
                ..
            }
        )));
    }

    #[test]
    fn test_oob_unknown_hook() {
        let mut d = ClaudeCodeDriver::new();
        let events = d.on_oob(oob("UnknownHook", serde_json::json!({})));
        assert!(events.is_empty());
    }

    #[test]
    fn test_oob_sets_hooks_active() {
        let mut d = ClaudeCodeDriver::new();
        assert!(!d.hooks_active);
        d.on_oob(oob("Stop", serde_json::json!({})));
        assert!(d.hooks_active);
    }

    #[test]
    fn test_detect_state_awaiting() {
        let mut d = ClaudeCodeDriver::new();
        d.parser_state = ParserState::AwaitingInput;
        let ctx = StateCtx {
            current_state: SessionState::Idle,
            last_activity_ms: 0,
            last_event: None,
            last_bytes_ms: 0,
            event_timestamps: vec![],
        };
        assert_eq!(d.detect_state(&ctx), Some(SessionState::Awaiting));
    }

    #[test]
    fn test_detect_state_idle_after_turn_finished() {
        let d = ClaudeCodeDriver::new();
        let ctx = StateCtx {
            current_state: SessionState::Streaming,
            last_activity_ms: 0,
            last_event: Some(AgentEvent::TurnFinished {
                turn_id: 1,
                tokens_used: 0,
                duration_ms: 0,
            }),
            last_bytes_ms: 0,
            event_timestamps: vec![],
        };
        assert_eq!(d.detect_state(&ctx), Some(SessionState::Idle));
    }

    #[test]
    fn test_detect_state_streaming() {
        let d = ClaudeCodeDriver::new();
        let ctx = StateCtx {
            current_state: SessionState::Idle,
            last_activity_ms: 0,
            last_event: Some(AgentEvent::TurnStarted {
                turn_id: 1,
                role: TurnRole::Assistant,
                content_start: None,
            }),
            last_bytes_ms: 0,
            event_timestamps: vec![],
        };
        assert_eq!(d.detect_state(&ctx), Some(SessionState::Streaming));
    }

    #[test]
    fn test_detect_state_none_when_already_correct() {
        let mut d = ClaudeCodeDriver::new();
        d.parser_state = ParserState::AwaitingInput;
        let ctx = StateCtx {
            current_state: SessionState::Awaiting,
            last_activity_ms: 0,
            last_event: None,
            last_bytes_ms: 0,
            event_timestamps: vec![],
        };
        assert_eq!(d.detect_state(&ctx), None);
    }
}
