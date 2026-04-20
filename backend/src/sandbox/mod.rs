#[cfg(feature = "sandbox")]
pub mod manager;
pub mod types;

#[cfg(feature = "sandbox")]
pub use manager::SandboxManager;
pub use types::{
    EgressProto, EgressRule, FsDiffEntry, FsDiffKind, FsDiffResponse, SandboxSpec, SandboxState,
    SandboxStatus,
};

// Stub SandboxManager when sandbox feature is disabled
#[cfg(not(feature = "sandbox"))]
pub struct SandboxManager {
    pub overlay_base: std::path::PathBuf,
    pub restic_repo: Option<String>,
}

#[cfg(not(feature = "sandbox"))]
impl SandboxManager {
    pub fn new(overlay_base: std::path::PathBuf, restic_repo: Option<String>) -> Self {
        SandboxManager {
            overlay_base,
            restic_repo,
        }
    }

    pub async fn create_container(
        &self,
        _session_id: &crate::session::SessionId,
        _spec: &SandboxSpec,
        _project_dir: &std::path::Path,
    ) -> anyhow::Result<SandboxStatus> {
        anyhow::bail!("Sandbox support not compiled (requires --features sandbox)")
    }

    pub async fn stop_container(&self, _session_id: &crate::session::SessionId) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn cleanup_overlay_dirs(&self, _session_id: &crate::session::SessionId) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn get_fs_diff(
        &self,
        _status: &SandboxStatus,
        _limit: usize,
        _offset: usize,
    ) -> anyhow::Result<FsDiffResponse> {
        anyhow::bail!("Sandbox support not compiled (requires --features sandbox)")
    }

    pub async fn merge_overlay(&self, _status: &SandboxStatus) -> anyhow::Result<String> {
        anyhow::bail!("Sandbox support not compiled (requires --features sandbox)")
    }

    pub async fn startup_cleanup(&self, _db: &sqlx::SqlitePool) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn ensure_restic_repo(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
