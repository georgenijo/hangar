// SYNTHETIC: fixtures are synthetic approximations of real CC PTY output.
// Real CC output patterns should be verified against a live instance before shipping.

use hangard::{drivers::claude_code::ClaudeCodeDriver, drivers::AgentDriver, events::AgentEvent};
use std::path::Path;

#[derive(Debug, serde::Deserialize)]
struct ExpectedEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pct_used: Option<f32>,
}

fn variant_name(e: &AgentEvent) -> &'static str {
    match e {
        AgentEvent::TurnStarted { .. } => "turn_started",
        AgentEvent::TurnFinished { .. } => "turn_finished",
        AgentEvent::ThinkingBlock { .. } => "thinking_block",
        AgentEvent::ToolCallStarted { .. } => "tool_call_started",
        AgentEvent::ToolCallFinished { .. } => "tool_call_finished",
        AgentEvent::AwaitingPermission { .. } => "awaiting_permission",
        AgentEvent::ModelChanged { .. } => "model_changed",
        AgentEvent::Error { .. } => "error",
        AgentEvent::ContextWindowSizeChanged { .. } => "context_window_size_changed",
        AgentEvent::SandboxStateChanged { .. } => "sandbox_state_changed",
        AgentEvent::SandboxMerged { .. } => "sandbox_merged",
    }
}

fn load_fixture(name: &str) -> (Vec<u8>, Vec<ExpectedEvent>) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cc_transcripts");
    let txt = std::fs::read(base.join(format!("{name}.txt")))
        .unwrap_or_else(|e| panic!("missing fixture {name}.txt: {e}"));
    let json_bytes = std::fs::read(base.join(format!("{name}.expected.json")))
        .unwrap_or_else(|e| panic!("missing fixture {name}.expected.json: {e}"));
    let expected: Vec<ExpectedEvent> = serde_json::from_slice(&json_bytes)
        .unwrap_or_else(|e| panic!("bad json in {name}.expected.json: {e}"));
    (txt, expected)
}

fn run_fixture(name: &str) -> (Vec<AgentEvent>, Vec<ExpectedEvent>) {
    let (bytes, expected) = load_fixture(name);
    let mut driver = ClaudeCodeDriver::new();
    let events = driver.on_bytes(&bytes);
    (events, expected)
}

fn check_fixture(name: &str) -> bool {
    let (events, expected) = run_fixture(name);

    for (i, exp) in expected.iter().enumerate() {
        let matching = events.iter().any(|e| {
            if variant_name(e) != exp.event_type {
                return false;
            }
            // Check optional fields
            if let Some(ref model) = exp.model {
                if let AgentEvent::ModelChanged { model: m } = e {
                    if m != model {
                        return false;
                    }
                }
            }
            if let Some(ref tool) = exp.tool {
                match e {
                    AgentEvent::ToolCallStarted { tool: t, .. } if t != tool => return false,
                    AgentEvent::AwaitingPermission { tool: t, .. } if t != tool => return false,
                    _ => {}
                }
            }
            if let Some(ref role) = exp.role {
                if let AgentEvent::TurnStarted { role: r, .. } = e {
                    let role_str = match r {
                        hangard::events::TurnRole::User => "user",
                        hangard::events::TurnRole::Assistant => "assistant",
                        hangard::events::TurnRole::System => "system",
                    };
                    if role_str != role {
                        return false;
                    }
                }
            }
            true
        });
        if !matching {
            eprintln!(
                "fixture {name}: expected event[{i}] {:?} not found in {:?}",
                exp.event_type,
                events.iter().map(variant_name).collect::<Vec<_>>()
            );
            return false;
        }
    }
    true
}

const FIXTURES: &[&str] = &[
    "model_line_sonnet",
    "model_line_opus",
    "model_line_with_ansi",
    "permission_simple",
    "permission_tool_use",
    "permission_denied",
    "context_window_50pct",
    "context_window_95pct",
    "tool_read_file",
    "tool_bash_command",
    "tool_edit_file",
    "tool_multiple_sequential",
    "turn_simple_qa",
    "turn_multi_tool",
    "turn_empty_response",
    "thinking_visible",
    "thinking_with_tools",
    "error_api_error",
    "error_rate_limit",
    "partial_ansi_sequence",
];

#[test]
fn test_fixture_classification_rate() {
    let mut passed = 0usize;
    let total = FIXTURES.len();

    for name in FIXTURES {
        if check_fixture(name) {
            passed += 1;
        } else {
            eprintln!("FAIL: {name}");
        }
    }

    let rate = passed as f64 / total as f64;
    eprintln!(
        "Classification rate: {passed}/{total} = {:.0}%",
        rate * 100.0
    );
    assert!(
        rate >= 0.95,
        "classification rate {:.0}% < 95% ({passed}/{total} passed)",
        rate * 100.0
    );
}

#[test]
fn test_partial_chunks_same_result() {
    let (bytes, _expected) = load_fixture("model_line_sonnet");

    // Full delivery
    let mut d1 = ClaudeCodeDriver::new();
    let full_events = d1.on_bytes(&bytes);

    // Chunked delivery (1 byte at a time)
    let mut d2 = ClaudeCodeDriver::new();
    let mut chunked_events = Vec::new();
    for byte in &bytes {
        chunked_events.extend(d2.on_bytes(&[*byte]));
    }
    // Flush remaining buffer with newline
    chunked_events.extend(d2.on_bytes(b"\n"));

    assert_eq!(
        full_events.iter().map(variant_name).collect::<Vec<_>>(),
        chunked_events.iter().map(variant_name).collect::<Vec<_>>(),
        "chunked delivery should produce same events as full delivery"
    );
}

#[test]
fn test_empty_bytes_returns_empty() {
    let mut d = ClaudeCodeDriver::new();
    let events = d.on_bytes(b"");
    assert!(events.is_empty());
}

#[test]
fn test_spawn_cfg_builds_command() {
    use hangard::{
        drivers::SpawnRequest,
        session::{SessionId, SessionKind},
    };
    use std::collections::HashMap;

    let d = ClaudeCodeDriver::new();
    let req = SpawnRequest {
        session_id: SessionId::new(),
        cwd: std::path::PathBuf::from("/tmp"),
        env: HashMap::new(),
        kind: SessionKind::ClaudeCode {
            config_override: None,
            project_dir: None,
        },
        hmac_key: vec![0u8; 32],
    };
    let cfg = d.spawn_cfg(&req).unwrap();
    assert!(cfg.command.contains(&"claude".to_string()));
    assert!(cfg.command.contains(&"--config-dir".to_string()));
    assert!(!cfg.temp_files.is_empty());

    // Verify settings.json was written
    let settings_path = cfg.temp_files[0].join("settings.json");
    let content = std::fs::read_to_string(&settings_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(v["hooks"]["PreToolUse"].is_array());
    assert!(v["hooks"]["Stop"].is_array());
}
