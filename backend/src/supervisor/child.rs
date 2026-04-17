use std::collections::HashMap;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use anyhow::Result;

use crate::supervisor::protocol::SessionInfo;

#[cfg(target_os = "linux")]
pub fn set_child_subreaper() -> Result<()> {
    let ret = unsafe {
        libc::prctl(
            libc::PR_SET_CHILD_SUBREAPER,
            1usize,
            0usize,
            0usize,
            0usize,
        )
    };
    if ret == -1 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn set_child_subreaper() -> Result<()> {
    anyhow::bail!("prctl PR_SET_CHILD_SUBREAPER not available on this platform")
}

pub enum MasterHandle {
    OwnedFd(OwnedFd),
}

impl MasterHandle {
    pub fn as_raw_fd(&self) -> RawFd {
        match self {
            MasterHandle::OwnedFd(fd) => fd.as_raw_fd(),
        }
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let fd = self.as_raw_fd();
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ret = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &ws) };
        if ret == -1 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }

    pub fn write_all(&self, data: &[u8]) -> Result<()> {
        use std::io::Write;
        let fd = self.as_raw_fd();
        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
        let result = file.write_all(data);
        // IMPORTANT: don't let File close the fd on drop
        std::mem::forget(file);
        result.map_err(Into::into)
    }
}

pub struct ManagedChild {
    pub id: String,
    pub slug: String,
    pub pid: u32,
    pub master: MasterHandle,
    pub sidecar_path: PathBuf,
}

impl ManagedChild {
    pub fn delete_sidecar(&self) {
        let _ = std::fs::remove_file(&self.sidecar_path);
    }

    pub fn to_session_info(&self) -> SessionInfo {
        // Check if process is still alive
        let running = unsafe { libc::kill(self.pid as i32, 0) } == 0;
        SessionInfo {
            id: self.id.clone(),
            slug: self.slug.clone(),
            pid: self.pid,
            running,
        }
    }
}

struct ChildTableInner {
    by_id: HashMap<String, ManagedChild>,
    by_pid: HashMap<u32, String>,
}

pub struct ChildTable {
    inner: Mutex<ChildTableInner>,
    state_dir: PathBuf,
}

impl ChildTable {
    pub fn new(state_dir: PathBuf) -> Self {
        Self {
            inner: Mutex::new(ChildTableInner {
                by_id: HashMap::new(),
                by_pid: HashMap::new(),
            }),
            state_dir,
        }
    }

    pub fn insert(&self, child: ManagedChild) {
        let mut inner = self.inner.lock().unwrap();
        inner.by_pid.insert(child.pid, child.id.clone());
        inner.by_id.insert(child.id.clone(), child);
    }

    pub fn remove(&self, id: &str) -> Option<ManagedChild> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(child) = inner.by_id.remove(id) {
            inner.by_pid.remove(&child.pid);
            Some(child)
        } else {
            None
        }
    }

    pub fn remove_by_pid(&self, pid: u32) -> Option<ManagedChild> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(id) = inner.by_pid.remove(&pid) {
            inner.by_id.remove(&id)
        } else {
            None
        }
    }

    pub fn with_child<F, R>(&self, id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&ManagedChild) -> R,
    {
        let inner = self.inner.lock().unwrap();
        inner.by_id.get(id).map(f)
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        let inner = self.inner.lock().unwrap();
        inner.by_id.values().map(|c| c.to_session_info()).collect()
    }

    pub fn sidecar_path(&self, id: &str) -> PathBuf {
        self.state_dir.join(format!("{}.json", id))
    }

    pub fn write_sidecar(&self, id: &str, slug: &str, pid: u32) -> Result<()> {
        std::fs::create_dir_all(&self.state_dir)?;
        let path = self.sidecar_path(id);
        let content = serde_json::json!({
            "id": id,
            "slug": slug,
            "pid": pid,
            "started_at": chrono::Utc::now().to_rfc3339(),
        });
        std::fs::write(&path, serde_json::to_string(&content)?)?;
        Ok(())
    }

    pub fn scan_orphans(&self) -> Result<Vec<(String, String, u32)>> {
        let mut orphans = Vec::new();
        if !self.state_dir.exists() {
            return Ok(orphans);
        }
        for entry in std::fs::read_dir(&self.state_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        let id = val["id"].as_str().unwrap_or("").to_string();
                        let slug = val["slug"].as_str().unwrap_or("").to_string();
                        let pid = val["pid"].as_u64().unwrap_or(0) as u32;
                        if !id.is_empty() && pid > 0 {
                            orphans.push((id, slug, pid));
                        }
                    }
                }
            }
        }
        Ok(orphans)
    }
}

pub async fn reap_children(table: Arc<ChildTable>) {
    let mut sigchld =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::child()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to install SIGCHLD handler: {}", e);
                return;
            }
        };

    loop {
        sigchld.recv().await;
        loop {
            match nix::sys::wait::waitpid(
                nix::unistd::Pid::from_raw(-1),
                Some(nix::sys::wait::WaitPidFlag::WNOHANG),
            ) {
                Ok(nix::sys::wait::WaitStatus::Exited(pid, status)) => {
                    let pid_u32 = pid.as_raw() as u32;
                    if let Some(child) = table.remove_by_pid(pid_u32) {
                        child.delete_sidecar();
                        tracing::info!(
                            pid = pid_u32,
                            exit_status = status,
                            id = %child.id,
                            "child exited"
                        );
                    } else {
                        tracing::debug!(
                            pid = pid_u32,
                            exit_status = status,
                            "reaped unknown child"
                        );
                    }
                }
                Ok(nix::sys::wait::WaitStatus::Signaled(pid, sig, _)) => {
                    let pid_u32 = pid.as_raw() as u32;
                    if let Some(child) = table.remove_by_pid(pid_u32) {
                        child.delete_sidecar();
                        tracing::info!(
                            pid = pid_u32,
                            signal = ?sig,
                            id = %child.id,
                            "child killed by signal"
                        );
                    }
                }
                Ok(nix::sys::wait::WaitStatus::StillAlive) => break,
                Ok(_) => continue,
                Err(nix::errno::Errno::ECHILD) => break,
                Err(e) => {
                    tracing::warn!("waitpid error: {}", e);
                    break;
                }
            }
        }
    }
}
