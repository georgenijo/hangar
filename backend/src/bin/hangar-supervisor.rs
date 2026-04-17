use std::collections::BTreeMap;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::{AsRawFd, OwnedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Semaphore;
use uuid::Uuid;

use hangar_backend::supervisor::child::{
    reap_children, set_child_subreaper, ChildTable, ManagedChild, MasterHandle,
};
use hangar_backend::supervisor::fd_pass;
use hangar_backend::supervisor::protocol::*;

struct SharedState {
    children: Arc<ChildTable>,
    pending_clients: Arc<Mutex<HashMap<Uuid, PendingClient>>>,
}

enum PendingClient {
    HasFdChannel(std::os::unix::net::UnixStream),
}

#[tokio::main]
async fn main() -> Result<()> {
    let socket_path_str = {
        let args: Vec<String> = std::env::args().collect();
        let mut result = None;
        for i in 0..args.len() {
            if args[i] == "--socket-path" {
                if let Some(v) = args.get(i + 1) {
                    result = Some(v.clone());
                    break;
                }
            }
        }
        result.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            format!("{}/.local/state/hangar/supervisor.sock", home)
        })
    };

    let socket_path = PathBuf::from(&socket_path_str);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("hangar-supervisor starting");

    set_child_subreaper()?;
    tracing::info!("set as child subreaper");

    // Create state dir
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let sidecar_dir = PathBuf::from(format!("{}/.local/state/hangar/supervisor", home));
    std::fs::create_dir_all(&sidecar_dir)?;

    // Remove stale socket
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let children = Arc::new(ChildTable::new(sidecar_dir.clone()));

    // Scan for orphans from previous run
    match children.scan_orphans() {
        Ok(orphans) => {
            for (id, slug, pid) in orphans {
                let alive = unsafe { libc::kill(pid as i32, 0) } == 0;
                if alive {
                    tracing::warn!(
                        id = %id,
                        slug = %slug,
                        pid = pid,
                        "orphan process from previous run — waiting for RegisterFd within 30s"
                    );
                } else {
                    tracing::info!(id = %id, "orphan process dead, cleaning sidecar");
                    let _ = std::fs::remove_file(sidecar_dir.join(format!("{}.json", id)));
                }
            }
        }
        Err(e) => tracing::warn!("failed to scan orphans: {}", e),
    }

    let listener = UnixListener::bind(&socket_path)?;
    std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o700))?;
    tracing::info!(socket = %socket_path.display(), "listening");

    let state = Arc::new(SharedState {
        children: children.clone(),
        pending_clients: Arc::new(Mutex::new(HashMap::new())),
    });

    let semaphore = Arc::new(Semaphore::new(16));

    // Spawn child reaper
    tokio::spawn(reap_children(children.clone()));

    // Heartbeat task
    {
        let children = children.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let sessions = children.list();
                tracing::info!(session_count = sessions.len(), "supervisor heartbeat");
            }
        });
    }

    // Signal handler for graceful shutdown
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result?;
                let permit = semaphore.clone().acquire_owned().await?;
                let state = state.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    if let Err(e) = handle_connection(stream, state).await {
                        tracing::warn!("connection error: {}", e);
                    }
                });
            }
            _ = sigterm.recv() => {
                tracing::info!("SIGTERM received, shutting down supervisor (children survive)");
                let _ = std::fs::remove_file(&socket_path);
                std::process::exit(0);
            }
            _ = sigint.recv() => {
                tracing::info!("SIGINT received, shutting down supervisor (children survive)");
                let _ = std::fs::remove_file(&socket_path);
                std::process::exit(0);
            }
        }
    }
}

async fn handle_connection(mut stream: UnixStream, state: Arc<SharedState>) -> Result<()> {
    // Read handshake
    let frame = read_frame(&mut stream).await?;
    let handshake: Handshake = serde_json::from_slice(&frame)?;

    // Send ack
    write_frame(&mut stream, b"{}").await?;

    match handshake.role {
        HandshakeRole::Primary => handle_primary(stream, handshake.client_id, state).await,
        HandshakeRole::FdChannel => handle_fd_channel(stream, handshake.client_id, state).await,
    }
}

async fn handle_fd_channel(
    stream: UnixStream,
    client_id: Uuid,
    state: Arc<SharedState>,
) -> Result<()> {
    let std_stream = stream.into_std()?;
    {
        let mut pending = state.pending_clients.lock().unwrap();
        pending.insert(client_id, PendingClient::HasFdChannel(std_stream));
    }
    // The stream is now owned by the pending map.
    // The Primary handler will use the raw fd and clean up the entry.
    // This task returns, but the fd stays alive via the map.
    Ok(())
}

async fn handle_primary(
    mut stream: UnixStream,
    client_id: Uuid,
    state: Arc<SharedState>,
) -> Result<()> {
    // Wait for FdChannel to appear (up to 5s)
    let fd_stream_fd: RawFd;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        {
            let pending = state.pending_clients.lock().unwrap();
            if let Some(PendingClient::HasFdChannel(std_stream)) = pending.get(&client_id) {
                fd_stream_fd = std_stream.as_raw_fd();
                break;
            }
        }
        if std::time::Instant::now() >= deadline {
            anyhow::bail!("timeout waiting for FdChannel from client {}", client_id);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // Command dispatch loop
    loop {
        let frame = match read_frame(&mut stream).await {
            Ok(f) => f,
            Err(_) => break,
        };
        let cmd: SupervisorCmd = match serde_json::from_slice(&frame) {
            Ok(c) => c,
            Err(e) => {
                let resp = SupervisorResp::Error {
                    message: format!("parse error: {}", e),
                };
                let _ = write_frame(&mut stream, &serde_json::to_vec(&resp)?).await;
                continue;
            }
        };

        let resp = dispatch_cmd(cmd, fd_stream_fd, &state).await;
        let resp_data = serde_json::to_vec(&resp)?;
        if let Err(e) = write_frame(&mut stream, &resp_data).await {
            tracing::debug!("write error on primary stream: {}", e);
            break;
        }
    }

    // Cleanup
    state.pending_clients.lock().unwrap().remove(&client_id);
    Ok(())
}

async fn dispatch_cmd(
    cmd: SupervisorCmd,
    fd_stream_fd: RawFd,
    state: &SharedState,
) -> SupervisorResp {
    match cmd {
        SupervisorCmd::Spawn {
            id,
            slug,
            command,
            env,
            cwd,
            cols,
            rows,
        } => handle_spawn(id, slug, command, env, cwd, cols, rows, state).await,
        SupervisorCmd::AttachFd { id, nonce } => {
            handle_attach_fd(&id, nonce, fd_stream_fd, state)
        }
        SupervisorCmd::Write { id, data } => handle_write(&id, data.into_bytes(), state),
        SupervisorCmd::Resize { id, cols, rows } => handle_resize(&id, cols, rows, state),
        SupervisorCmd::Kill { id, signal } => handle_kill(&id, signal, state),
        SupervisorCmd::List => SupervisorResp::Sessions(state.children.list()),
        SupervisorCmd::RegisterFd { id, pid } => {
            handle_register_fd(&id, pid, fd_stream_fd, state)
        }
    }
}

async fn handle_spawn(
    id: String,
    slug: String,
    command: Vec<String>,
    env: BTreeMap<String, String>,
    cwd: String,
    cols: u16,
    rows: u16,
    state: &SharedState,
) -> SupervisorResp {
    if command.is_empty() {
        return SupervisorResp::Error {
            message: "command is empty".to_string(),
        };
    }

    let ws = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let pty_result = match nix::pty::openpty(Some(&ws), None) {
        Ok(r) => r,
        Err(e) => {
            return SupervisorResp::Error {
                message: format!("openpty: {}", e),
            }
        }
    };

    let master: OwnedFd = pty_result.master;
    let slave: OwnedFd = pty_result.slave;
    let slave_raw = slave.as_raw_fd();

    let mut cmd = std::process::Command::new(&command[0]);
    if command.len() > 1 {
        cmd.args(&command[1..]);
    }
    if !cwd.is_empty() {
        cmd.current_dir(&cwd);
    }
    for (k, v) in &env {
        cmd.env(k, v);
    }
    if !env.contains_key("TERM") {
        cmd.env("TERM", "xterm-256color");
    }

    unsafe {
        cmd.pre_exec(move || {
            nix::unistd::setsid()
                .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
            libc::ioctl(slave_raw, libc::TIOCSCTTY, 0i32);
            nix::unistd::dup2(slave_raw, 0)
                .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
            nix::unistd::dup2(slave_raw, 1)
                .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
            nix::unistd::dup2(slave_raw, 2)
                .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;
            if slave_raw > 2 {
                let _ = nix::unistd::close(slave_raw);
            }
            Ok(())
        });
    }

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return SupervisorResp::Error {
                message: format!("spawn: {}", e),
            }
        }
    };

    let pid = child.id();
    // Drop child struct (doesn't kill process, just drops handles)
    drop(child);
    // Close slave in parent (child has its own copy)
    drop(slave);

    let sidecar_path = state.children.sidecar_path(&id);
    if let Err(e) = state.children.write_sidecar(&id, &slug, pid) {
        tracing::warn!("failed to write sidecar for {}: {}", id, e);
    }

    state.children.insert(ManagedChild {
        id: id.clone(),
        slug,
        pid,
        master: MasterHandle::OwnedFd(master),
        sidecar_path,
    });

    tracing::info!(id = %id, pid = pid, "spawned session");
    SupervisorResp::Spawned { id, pid }
}

fn handle_attach_fd(
    id: &str,
    nonce: u64,
    fd_stream_fd: RawFd,
    state: &SharedState,
) -> SupervisorResp {
    let result = state.children.with_child(id, |child| child.master.as_raw_fd());
    match result {
        Some(master_fd) => {
            let payload = serde_json::to_vec(&serde_json::json!({"nonce": nonce}))
                .unwrap_or_else(|_| b"{}".to_vec());
            if let Err(e) = fd_pass::send_fd(fd_stream_fd, master_fd, &payload) {
                return SupervisorResp::Error {
                    message: format!("send_fd: {}", e),
                };
            }
            SupervisorResp::FdReady { nonce }
        }
        None => SupervisorResp::Error {
            message: format!("session {} not found", id),
        },
    }
}

fn handle_write(id: &str, data: Vec<u8>, state: &SharedState) -> SupervisorResp {
    match state.children.with_child(id, |child| child.master.write_all(&data)) {
        Some(Ok(())) => SupervisorResp::Written,
        Some(Err(e)) => SupervisorResp::Error {
            message: format!("write: {}", e),
        },
        None => SupervisorResp::Error {
            message: format!("session {} not found", id),
        },
    }
}

fn handle_resize(id: &str, cols: u16, rows: u16, state: &SharedState) -> SupervisorResp {
    match state
        .children
        .with_child(id, |child| child.master.resize(cols, rows))
    {
        Some(Ok(())) => SupervisorResp::Resized,
        Some(Err(e)) => SupervisorResp::Error {
            message: format!("resize: {}", e),
        },
        None => SupervisorResp::Error {
            message: format!("session {} not found", id),
        },
    }
}

fn handle_kill(id: &str, signal: i32, state: &SharedState) -> SupervisorResp {
    match state.children.with_child(id, |child| {
        let ret = unsafe { libc::kill(child.pid as i32, signal) };
        if ret == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }) {
        Some(Ok(())) => SupervisorResp::Killed,
        Some(Err(e)) => SupervisorResp::Error {
            message: format!("kill: {}", e),
        },
        None => SupervisorResp::Error {
            message: format!("session {} not found", id),
        },
    }
}

fn handle_register_fd(
    id: &str,
    pid: u32,
    fd_stream_fd: RawFd,
    state: &SharedState,
) -> SupervisorResp {
    let mut buf = vec![0u8; 256];
    let (_n, fd_opt) = match fd_pass::recv_fd(fd_stream_fd, &mut buf) {
        Ok(r) => r,
        Err(e) => {
            return SupervisorResp::Error {
                message: format!("recv_fd: {}", e),
            }
        }
    };
    let owned_fd = match fd_opt {
        Some(f) => f,
        None => {
            return SupervisorResp::Error {
                message: "no fd received".to_string(),
            }
        }
    };

    // Verify pid is alive
    let alive = unsafe { libc::kill(pid as i32, 0) } == 0;
    if !alive {
        return SupervisorResp::Error {
            message: format!("pid {} is not alive", pid),
        };
    }

    // Check if this matches an orphan sidecar
    let sidecar_path = state.children.sidecar_path(id);
    let slug = if sidecar_path.exists() {
        std::fs::read_to_string(&sidecar_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v["slug"].as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| id.to_string())
    } else {
        id.to_string()
    };

    state.children.insert(ManagedChild {
        id: id.to_string(),
        slug: slug.clone(),
        pid,
        master: MasterHandle::OwnedFd(owned_fd),
        sidecar_path: sidecar_path.clone(),
    });

    // Write/update sidecar
    if let Err(e) = state.children.write_sidecar(id, &slug, pid) {
        tracing::warn!("failed to write sidecar on register_fd: {}", e);
    }

    tracing::info!(id = %id, pid = pid, "registered fd for orphaned session");
    SupervisorResp::Spawned {
        id: id.to_string(),
        pid,
    }
}
