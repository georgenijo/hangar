use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NtfyPriority {
    High,
    Normal,
    Low,
}

impl NtfyPriority {
    pub fn as_ntfy_str(&self) -> &'static str {
        match self {
            NtfyPriority::High => "4",
            NtfyPriority::Normal => "3",
            NtfyPriority::Low => "2",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PushRule {
    pub name: String,
    pub enabled: bool,
    pub priority: NtfyPriority,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PushConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_ntfy_url")]
    pub ntfy_url: String,
    #[serde(default = "default_ntfy_topic")]
    pub ntfy_topic: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_rules")]
    pub rules: Vec<PushRule>,
}

fn default_true() -> bool {
    true
}

fn default_ntfy_url() -> String {
    "http://localhost:2586".to_string()
}

fn default_ntfy_topic() -> String {
    "hangar".to_string()
}

fn default_base_url() -> String {
    std::env::var("HANGAR_BASE_URL").unwrap_or_else(|_| "https://localhost:8080".to_string())
}

fn default_rules() -> Vec<PushRule> {
    vec![
        PushRule {
            name: "awaiting_permission".to_string(),
            enabled: true,
            priority: NtfyPriority::High,
        },
        PushRule {
            name: "agent_error".to_string(),
            enabled: true,
            priority: NtfyPriority::High,
        },
        PushRule {
            name: "session_exited_nonzero".to_string(),
            enabled: true,
            priority: NtfyPriority::Normal,
        },
        PushRule {
            name: "context_window_80pct".to_string(),
            enabled: true,
            priority: NtfyPriority::Normal,
        },
        PushRule {
            name: "high_token_burn".to_string(),
            enabled: true,
            priority: NtfyPriority::Normal,
        },
        PushRule {
            name: "approaching_context_window".to_string(),
            enabled: true,
            priority: NtfyPriority::High,
        },
    ]
}

impl Default for PushConfig {
    fn default() -> Self {
        PushConfig {
            enabled: true,
            ntfy_url: default_ntfy_url(),
            ntfy_topic: default_ntfy_topic(),
            base_url: default_base_url(),
            rules: default_rules(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSourceKind {
    Journalctl,
    Unit,
    File,
    PaneScrollback,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogSourceConfig {
    pub name: String,
    pub kind: LogSourceKind,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

fn default_tail_lines() -> usize {
    500
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_tail_lines")]
    pub tail_lines: usize,
    #[serde(default)]
    pub sources: Vec<LogSourceConfig>,
}

impl Default for LogsConfig {
    fn default() -> Self {
        LogsConfig {
            enabled: false,
            tail_lines: 500,
            sources: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HangarConfig {
    #[serde(default)]
    pub push: PushConfig,
    #[serde(default)]
    pub logs: LogsConfig,
}

pub fn load() -> Result<HangarConfig> {
    let config_dir =
        dirs::config_dir().ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?;
    let config_path = config_dir.join("hangar").join("config.toml");

    if !config_path.exists() {
        tracing::info!("no config file at {:?}, using defaults", config_path);
        return Ok(HangarConfig::default());
    }

    let contents = std::fs::read_to_string(&config_path)
        .with_context(|| format!("reading {:?}", config_path))?;

    let config: HangarConfig =
        toml::from_str(&contents).with_context(|| format!("parsing {:?}", config_path))?;

    Ok(config)
}
