use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSpec {
    #[serde(default = "default_image")]
    pub image: String,
    #[serde(default)]
    pub cpu_quota: Option<f64>,
    #[serde(default)]
    pub memory_limit_mb: Option<u64>,
    #[serde(default)]
    pub egress_allowlist: Vec<EgressRule>,
    #[serde(default)]
    pub allocate_tty: bool,
}

fn default_image() -> String {
    "ubuntu:24.04".to_string()
}

impl Default for SandboxSpec {
    fn default() -> Self {
        SandboxSpec {
            image: default_image(),
            cpu_quota: None,
            memory_limit_mb: None,
            egress_allowlist: vec![],
            allocate_tty: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressRule {
    pub host: String,
    pub port: u16,
    pub proto: EgressProto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EgressProto {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SandboxState {
    Creating,
    Running,
    Stopped,
    Merging,
    Merged,
    Failed { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxStatus {
    pub spec: SandboxSpec,
    pub state: SandboxState,
    pub container_name: String,
    pub overlay_dir: PathBuf,
    pub project_dir: PathBuf,
    pub merged_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsDiffEntry {
    pub path: String,
    pub kind: FsDiffKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FsDiffKind {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsDiffResponse {
    pub entries: Vec<FsDiffEntry>,
    pub total: usize,
    pub truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_spec_round_trip() {
        let spec = SandboxSpec {
            image: "ubuntu:24.04".to_string(),
            cpu_quota: Some(2.0),
            memory_limit_mb: Some(4096),
            egress_allowlist: vec![EgressRule {
                host: "api.anthropic.com".to_string(),
                port: 443,
                proto: EgressProto::Tcp,
            }],
            allocate_tty: false,
        };
        let json = serde_json::to_string(&spec).unwrap();
        let spec2: SandboxSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec2.image, spec.image);
        assert_eq!(spec2.cpu_quota, spec.cpu_quota);
        assert_eq!(spec2.memory_limit_mb, spec.memory_limit_mb);
        assert_eq!(spec2.allocate_tty, spec.allocate_tty);
        assert_eq!(spec2.egress_allowlist.len(), 1);
    }

    #[test]
    fn sandbox_state_failed_round_trip() {
        let state = SandboxState::Failed {
            message: "container OOM".to_string(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let state2: SandboxState = serde_json::from_str(&json).unwrap();
        assert!(matches!(state2, SandboxState::Failed { .. }));
        if let SandboxState::Failed { message } = state2 {
            assert_eq!(message, "container OOM");
        }
    }

    #[test]
    fn sandbox_status_round_trip() {
        let status = SandboxStatus {
            spec: SandboxSpec::default(),
            state: SandboxState::Running,
            container_name: "hangar-test123".to_string(),
            overlay_dir: PathBuf::from("/tmp/overlays/test"),
            project_dir: PathBuf::from("/home/user/project"),
            merged_dir: PathBuf::from("/tmp/overlays/test/merged"),
        };
        let json = serde_json::to_string(&status).unwrap();
        let status2: SandboxStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status2.container_name, status.container_name);
        assert_eq!(status2.project_dir, status.project_dir);
    }

    #[test]
    fn sandbox_none_in_session_json() {
        #[derive(Serialize, Deserialize)]
        struct FakeSession {
            id: String,
            sandbox: Option<SandboxStatus>,
        }
        let s = FakeSession {
            id: "abc".to_string(),
            sandbox: None,
        };
        let json = serde_json::to_string(&s).unwrap();
        let s2: FakeSession = serde_json::from_str(&json).unwrap();
        assert!(s2.sandbox.is_none());
    }
}
