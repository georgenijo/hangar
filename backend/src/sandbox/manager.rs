#![cfg(feature = "sandbox")]

use std::os::unix::fs::FileTypeExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use tracing::{info, warn};

use super::types::{
    EgressProto, EgressRule, FsDiffEntry, FsDiffKind, FsDiffResponse, SandboxSpec, SandboxState,
    SandboxStatus,
};
use crate::session::{Session, SessionId, SessionState};

pub struct SandboxManager {
    pub overlay_base: PathBuf,
    pub restic_repo: Option<String>,
}

impl SandboxManager {
    pub fn new(overlay_base: PathBuf, restic_repo: Option<String>) -> Self {
        SandboxManager {
            overlay_base,
            restic_repo,
        }
    }

    pub async fn create_container(
        &self,
        session_id: &SessionId,
        spec: &SandboxSpec,
        project_dir: &Path,
    ) -> Result<SandboxStatus> {
        let overlay_dir = self.overlay_base.join(session_id.as_ref());
        let upper_dir = overlay_dir.join("upper");
        let work_dir = overlay_dir.join("work");
        let merged_dir = overlay_dir.join("merged");

        tokio::fs::create_dir_all(&upper_dir).await?;
        tokio::fs::create_dir_all(&work_dir).await?;
        tokio::fs::create_dir_all(&merged_dir).await?;

        let lower = project_dir.display().to_string();
        let upper = upper_dir.display().to_string();
        let work = work_dir.display().to_string();
        let merged = merged_dir.display().to_string();

        let mount_opts = format!("lowerdir={lower},upperdir={upper},workdir={work}");
        let status = tokio::process::Command::new("sudo")
            .args([
                "mount",
                "-t",
                "overlay",
                "overlay",
                "-o",
                &mount_opts,
                &merged,
            ])
            .status()
            .await
            .context("mount overlayfs")?;
        if !status.success() {
            anyhow::bail!("overlayfs mount failed");
        }

        let network = if spec.egress_allowlist.is_empty() {
            "--network=none".to_string()
        } else {
            "--network=bridge".to_string()
        };

        let container_name = format!("hangar-{session_id}");
        let project_str = project_dir.display().to_string();

        let mut args = vec![
            "podman".to_string(),
            "run".to_string(),
            "--name".to_string(),
            container_name.clone(),
            "--detach".to_string(),
            "-v".to_string(),
            format!("{merged}:{project_str}"),
            network,
            "--init".to_string(),
        ];

        if let Some(quota) = spec.cpu_quota {
            args.push(format!("--cpus={quota}"));
        }
        if let Some(mem_mb) = spec.memory_limit_mb {
            args.push(format!("--memory={mem_mb}m"));
        }

        args.push(spec.image.clone());
        args.push("sleep".to_string());
        args.push("infinity".to_string());

        let output = tokio::process::Command::new("sudo")
            .args(&args)
            .output()
            .await
            .context("podman run")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("podman run failed: {stderr}");
        }

        if !spec.egress_allowlist.is_empty() {
            apply_egress_rules(&container_name, &spec.egress_allowlist).await?;
        }

        Ok(SandboxStatus {
            spec: spec.clone(),
            state: SandboxState::Running,
            container_name,
            overlay_dir,
            project_dir: project_dir.to_path_buf(),
            merged_dir,
        })
    }

    pub async fn stop_container(&self, session_id: &SessionId) -> Result<()> {
        let container_name = format!("hangar-{session_id}");
        let overlay_dir = self.overlay_base.join(session_id.as_ref());
        let merged_dir = overlay_dir.join("merged");

        let exists = tokio::process::Command::new("sudo")
            .args(["podman", "container", "exists", &container_name])
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);

        if exists {
            let _ = tokio::process::Command::new("sudo")
                .args(["podman", "stop", "-t", "5", &container_name])
                .status()
                .await;
            let _ = tokio::process::Command::new("sudo")
                .args(["podman", "rm", "-f", &container_name])
                .status()
                .await;
        }

        let mounted = is_mounted(&merged_dir).await;
        if mounted {
            let status = tokio::process::Command::new("sudo")
                .args(["umount", &merged_dir.display().to_string()])
                .status()
                .await
                .context("umount overlay")?;
            if !status.success() {
                warn!(
                    "umount of {} failed (may already be unmounted)",
                    merged_dir.display()
                );
            }
        }

        Ok(())
    }

    pub fn cleanup_overlay_dirs(&self, session_id: &SessionId) -> Result<()> {
        let overlay_dir = self.overlay_base.join(session_id.as_ref());
        if overlay_dir.exists() {
            std::fs::remove_dir_all(&overlay_dir)
                .with_context(|| format!("removing overlay dir {:?}", overlay_dir))?;
        }
        Ok(())
    }

    pub async fn get_fs_diff(
        &self,
        status: &SandboxStatus,
        limit: usize,
        offset: usize,
    ) -> Result<FsDiffResponse> {
        let upper_dir = status.overlay_dir.join("upper");
        let all_entries = walk_upper(&upper_dir, &status.project_dir)?;
        let total = all_entries.len();
        let sliced: Vec<FsDiffEntry> = all_entries.into_iter().skip(offset).take(limit).collect();
        let truncated = total > offset + limit;
        Ok(FsDiffResponse {
            entries: sliced,
            total,
            truncated,
        })
    }

    pub async fn merge_overlay(&self, status: &SandboxStatus) -> Result<String> {
        let snapshot_id = if let Some(ref repo) = self.restic_repo {
            let project_str = status.project_dir.display().to_string();
            let output = tokio::process::Command::new("restic")
                .args([
                    "-r",
                    repo,
                    "backup",
                    "--json",
                    "--no-password",
                    &project_str,
                ])
                .env("RESTIC_PASSWORD", "")
                .output()
                .await
                .context("restic backup")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("restic backup failed: {stderr}");
            }

            parse_restic_snapshot_id(&String::from_utf8_lossy(&output.stdout))
        } else {
            warn!("no restic_repo configured; skipping backup before merge");
            String::new()
        };

        apply_upperdir_to_lowerdir(&status.overlay_dir.join("upper"), &status.project_dir).await?;

        Ok(snapshot_id)
    }

    pub async fn startup_cleanup(&self, db: &SqlitePool) -> Result<()> {
        let sessions = Session::list(db).await?;
        for session in sessions {
            let sandbox = match session.sandbox {
                Some(s) => s,
                None => continue,
            };

            let sid = &session.id;
            match sandbox.state {
                SandboxState::Running | SandboxState::Creating => {
                    info!("startup cleanup: stopping orphaned container for session {sid}");
                    let _ = self.stop_container(sid).await;
                    let stopped = SandboxStatus {
                        state: SandboxState::Stopped,
                        ..sandbox
                    };
                    let _ = Session::update_sandbox(db, sid.as_ref(), &stopped).await;
                    let _ = Session::update_state(db, sid, SessionState::Exited).await;
                }
                SandboxState::Merging => {
                    warn!(
                        "startup cleanup: session {sid} left in Merging state; manual intervention required"
                    );
                }
                SandboxState::Merged | SandboxState::Stopped | SandboxState::Failed { .. } => {
                    let _ = self.cleanup_overlay_dirs(sid);
                }
            }
        }
        Ok(())
    }

    pub async fn ensure_restic_repo(&self) -> Result<()> {
        let repo = match &self.restic_repo {
            Some(r) => r.clone(),
            None => return Ok(()),
        };
        let output = tokio::process::Command::new("restic")
            .args(["-r", &repo, "init", "--no-password"])
            .env("RESTIC_PASSWORD", "")
            .output()
            .await
            .context("restic init")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already initialized")
                || stderr.contains("config file already exists")
            {
                return Ok(());
            }
            anyhow::bail!("restic init failed: {stderr}");
        }
        Ok(())
    }
}

async fn apply_egress_rules(container_name: &str, rules: &[EgressRule]) -> Result<()> {
    let output = tokio::process::Command::new("sudo")
        .args([
            "podman",
            "inspect",
            "--format",
            "{{.State.Pid}}",
            container_name,
        ])
        .output()
        .await
        .context("podman inspect for pid")?;

    if !output.status.success() {
        anyhow::bail!("podman inspect failed for {container_name}");
    }

    let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let pid: u32 = pid_str
        .parse()
        .with_context(|| format!("parsing container pid: {pid_str}"))?;

    let netns = format!("/proc/{pid}/ns/net");

    let mut rule_lines = Vec::new();
    for rule in rules {
        let proto_str = match rule.proto {
            EgressProto::Tcp => "tcp",
            EgressProto::Udp => "udp",
        };
        let port = rule.port;
        let host = &rule.host;

        let addrs: Vec<std::net::IpAddr> = tokio::net::lookup_host(format!("{host}:{port}"))
            .await
            .with_context(|| format!("resolving {host}"))?
            .map(|sa| sa.ip())
            .collect();

        for ip in addrs {
            let ip_version = if ip.is_ipv4() { "ip" } else { "ip6" };
            rule_lines.push(format!(
                "        {ip_version} daddr {ip} {proto_str} dport {port} accept"
            ));
        }
    }

    let script = format!(
        "add table inet hangar_egress\n\
         flush table inet hangar_egress\n\
         add chain inet hangar_egress output {{ type filter hook output priority 0; policy drop; }}\n\
         add rule inet hangar_egress output oifname \"lo\" accept\n\
         add rule inet hangar_egress output ct state established,related accept\n\
         {rules}\n",
        rules = rule_lines.join("\n")
    );

    let mut cmd = tokio::process::Command::new("sudo");
    cmd.args(["nsenter", "--net", &netns, "nft", "-f", "-"]);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().context("spawn nsenter nft")?;
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(script.as_bytes()).await?;
    }
    let out = child.wait_with_output().await?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("nft rules failed: {stderr}");
    }

    Ok(())
}

async fn is_mounted(path: &Path) -> bool {
    let path_str = path.display().to_string();
    std::fs::read_to_string("/proc/mounts")
        .map(|contents| contents.lines().any(|line| line.contains(&path_str)))
        .unwrap_or(false)
}

fn walk_upper(upper_dir: &Path, lower_dir: &Path) -> Result<Vec<FsDiffEntry>> {
    let mut entries = Vec::new();

    if !upper_dir.exists() {
        return Ok(entries);
    }

    let mut stack = vec![upper_dir.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).with_context(|| format!("readdir {:?}", dir))? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            let file_type = entry.file_type()?;

            let rel_path = path
                .strip_prefix(upper_dir)
                .map_err(|_| anyhow::anyhow!("path not under upper_dir"))?
                .to_path_buf();

            if let Some(orig_name) = file_name_str.strip_prefix(".wh.") {
                if orig_name == ".wh..opq" {
                    continue;
                }
                let orig_rel = rel_path
                    .parent()
                    .map(|p| p.join(orig_name))
                    .unwrap_or_else(|| PathBuf::from(orig_name));
                entries.push(FsDiffEntry {
                    path: orig_rel.to_string_lossy().into_owned(),
                    kind: FsDiffKind::Deleted,
                });
                continue;
            }

            if file_type.is_char_device() {
                let metadata = entry.metadata()?;
                if metadata.rdev() == 0 {
                    entries.push(FsDiffEntry {
                        path: rel_path.to_string_lossy().into_owned(),
                        kind: FsDiffKind::Deleted,
                    });
                    continue;
                }
            }

            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            let lower_path = lower_dir.join(&rel_path);
            let kind = if lower_path.exists() {
                FsDiffKind::Modified
            } else {
                FsDiffKind::Added
            };

            entries.push(FsDiffEntry {
                path: rel_path.to_string_lossy().into_owned(),
                kind,
            });
        }
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

async fn apply_upperdir_to_lowerdir(upper_dir: &Path, lower_dir: &Path) -> Result<()> {
    if !upper_dir.exists() {
        return Ok(());
    }

    let mut stack = vec![upper_dir.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).with_context(|| format!("readdir {:?}", dir))? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            let file_type = entry.file_type()?;

            let rel_path = path
                .strip_prefix(upper_dir)
                .map_err(|_| anyhow::anyhow!("path not under upper_dir"))?;

            if file_name_str == ".wh..wh..opq" {
                continue;
            }

            if let Some(orig_name) = file_name_str.strip_prefix(".wh.") {
                let target = rel_path
                    .parent()
                    .map(|p| lower_dir.join(p).join(orig_name))
                    .unwrap_or_else(|| lower_dir.join(orig_name));
                if target.exists() {
                    if target.is_dir() {
                        tokio::fs::remove_dir_all(&target).await?;
                    } else {
                        tokio::fs::remove_file(&target).await?;
                    }
                }
                continue;
            }

            if file_type.is_char_device() {
                let metadata = entry.metadata()?;
                if metadata.rdev() == 0 {
                    let target = lower_dir.join(rel_path);
                    if target.exists() {
                        if target.is_dir() {
                            tokio::fs::remove_dir_all(&target).await?;
                        } else {
                            tokio::fs::remove_file(&target).await?;
                        }
                    }
                    continue;
                }
            }

            if file_type.is_dir() {
                let target = lower_dir.join(rel_path);
                tokio::fs::create_dir_all(&target).await?;
                stack.push(path);
                continue;
            }

            let target = lower_dir.join(rel_path);
            if let Some(parent) = target.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(&path, &target).await?;
        }
    }

    Ok(())
}

fn parse_restic_snapshot_id(output: &str) -> String {
    for line in output.lines().rev() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if v.get("message_type").and_then(|t| t.as_str()) == Some("summary") {
                if let Some(id) = v.get("snapshot_id").and_then(|i| i.as_str()) {
                    return id.to_string();
                }
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_manager(tmp: &TempDir) -> SandboxManager {
        SandboxManager::new(tmp.path().join("overlays"), None)
    }

    #[test]
    fn fsdiff_added_modified_deleted() {
        let tmp = tempfile::tempdir().unwrap();
        let upper = tmp.path().join("upper");
        let lower = tmp.path().join("lower");
        std::fs::create_dir_all(&upper).unwrap();
        std::fs::create_dir_all(&lower).unwrap();

        // new_file.txt: in upper only → Added
        std::fs::write(upper.join("new_file.txt"), b"new").unwrap();

        // existing.txt: in both → Modified
        std::fs::write(upper.join("existing.txt"), b"changed").unwrap();
        std::fs::write(lower.join("existing.txt"), b"original").unwrap();

        // .wh.deleted.txt: whiteout → Deleted
        std::fs::write(upper.join(".wh.deleted.txt"), b"").unwrap();

        let entries = walk_upper(&upper, &lower).unwrap();
        assert_eq!(entries.len(), 3);

        let deleted = entries.iter().find(|e| e.path == "deleted.txt").unwrap();
        assert!(matches!(deleted.kind, FsDiffKind::Deleted));

        let existing = entries.iter().find(|e| e.path == "existing.txt").unwrap();
        assert!(matches!(existing.kind, FsDiffKind::Modified));

        let new_f = entries.iter().find(|e| e.path == "new_file.txt").unwrap();
        assert!(matches!(new_f.kind, FsDiffKind::Added));
    }

    #[test]
    fn fsdiff_pagination() {
        let tmp = tempfile::tempdir().unwrap();
        let upper = tmp.path().join("upper");
        let lower = tmp.path().join("lower");
        std::fs::create_dir_all(&upper).unwrap();
        std::fs::create_dir_all(&lower).unwrap();

        for i in 0..10u8 {
            std::fs::write(upper.join(format!("file{i:02}.txt")), b"x").unwrap();
        }

        let entries = walk_upper(&upper, &lower).unwrap();
        assert_eq!(entries.len(), 10);

        // Test that pagination works via get_fs_diff
        let mgr = make_manager(&tmp);
        let status = SandboxStatus {
            spec: crate::sandbox::types::SandboxSpec::default(),
            state: crate::sandbox::types::SandboxState::Running,
            container_name: "test".to_string(),
            overlay_dir: tmp.path().to_path_buf(),
            project_dir: lower.clone(),
            merged_dir: tmp.path().join("merged"),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let resp = rt.block_on(mgr.get_fs_diff(&status, 3, 0)).unwrap();
        assert_eq!(resp.entries.len(), 3);
        assert_eq!(resp.total, 10);
        assert!(resp.truncated);

        let resp2 = rt.block_on(mgr.get_fs_diff(&status, 3, 9)).unwrap();
        assert_eq!(resp2.entries.len(), 1);
        assert!(!resp2.truncated);
    }

    #[test]
    fn parse_restic_output_empty() {
        let id = parse_restic_snapshot_id("some garbage\nno json here");
        assert_eq!(id, "");
    }

    #[test]
    fn parse_restic_output_summary() {
        let line = r#"{"message_type":"summary","snapshot_id":"abc12345","files_new":1}"#;
        let id = parse_restic_snapshot_id(line);
        assert_eq!(id, "abc12345");
    }

    #[tokio::test]
    #[ignore = "requires podman + overlayfs + root; run manually"]
    async fn test_full_container_lifecycle() {
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("project");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(project_dir.join("hello.txt"), b"hello").unwrap();

        let mgr = SandboxManager::new(tmp.path().join("overlays"), None);
        let session_id = crate::session::SessionId::new();
        let spec = crate::sandbox::types::SandboxSpec::default();

        let status = mgr
            .create_container(&session_id, &spec, &project_dir)
            .await
            .unwrap();
        assert!(matches!(status.state, SandboxState::Running));

        let diff = mgr.get_fs_diff(&status, 100, 0).await.unwrap();
        assert_eq!(diff.total, 0);

        mgr.stop_container(&session_id).await.unwrap();
        mgr.cleanup_overlay_dirs(&session_id).unwrap();
    }
}
