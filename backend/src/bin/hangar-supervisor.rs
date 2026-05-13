use std::collections::HashMap;
use std::os::unix::io::{AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Result};
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use tracing::{info, warn};

use hangard::supervisor_protocol::{
    hangar_state_dir, read_frame, send_fd, supervisor_sock_path, write_frame, SupervisorRequest,
    SupervisorResponse, SupervisorSessionInfo,
};

struct ManagedSession {
    session_id: String,
    master_fd: OwnedFd,
    child_pid: Pid,
    alive: bool,
}

struct SupervisorState {
    sessions: HashMap<String, ManagedSession>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    #[cfg(target_os = "linux")]
    unsafe {
        libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1usize, 0usize, 0usize, 0usize);
    }

    let state_dir = hangar_state_dir();
    std::fs::create_dir_all(&state_dir)?;

    let sock_path = supervisor_sock_path();
    if let Some(parent) = sock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _ = std::fs::remove_file(&sock_path);

    let listener = tokio::net::UnixListener::bind(&sock_path)?;
    info!("supervisor listening on {:?}", sock_path);

    // Set permissions to user-only
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&sock_path, std::fs::Permissions::from_mode(0o700))?;
    }

    let state = Arc::new(Mutex::new(SupervisorState {
        sessions: HashMap::new(),
    }));

    tokio::spawn(reap_children(Arc::clone(&state)));

    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let std_stream = match stream.into_std() {
                            Ok(s) => s,
                            Err(e) => { warn!("into_std failed: {e}"); continue; }
                        };
                        let state = Arc::clone(&state);
                        std::thread::spawn(move || {
                            if let Err(e) = handle_client(std_stream, state) {
                                tracing::debug!("client disconnected: {e}");
                            }
                        });
                    }
                    Err(e) => warn!("accept error: {e}"),
                }
            }
            _ = sigterm.recv() => {
                info!("SIGTERM received, shutting down");
                break;
            }
            _ = sigint.recv() => {
                info!("SIGINT received, shutting down");
                break;
            }
        }
    }

    let _ = std::fs::remove_file(&sock_path);
    Ok(())
}

async fn reap_children(state: Arc<Mutex<SupervisorState>>) {
    let mut sigchld = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::child())
        .expect("failed to register SIGCHLD handler");

    loop {
        sigchld.recv().await;
        loop {
            match waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(pid, _)) | Ok(WaitStatus::Signaled(pid, _, _)) => {
                    let mut s = state.lock().unwrap();
                    for session in s.sessions.values_mut() {
                        if session.child_pid == pid {
                            session.alive = false;
                            info!("session {} (pid {}) exited", session.session_id, pid);
                        }
                    }
                }
                Ok(WaitStatus::StillAlive) | Err(_) => break,
                _ => {}
            }
        }
    }
}

fn handle_client(mut stream: UnixStream, state: Arc<Mutex<SupervisorState>>) -> Result<()> {
    // tokio::net::UnixStream::into_std() does NOT reset the fd to blocking mode;
    // it just wraps the raw fd which tokio keeps non-blocking. Without this call,
    // read_exact returns WouldBlock after the first response and closes the connection.
    stream.set_nonblocking(false)?;
    loop {
        let frame = match read_frame(&mut stream) {
            Ok(f) => f,
            Err(_) => break,
        };

        let req: SupervisorRequest = serde_json::from_slice(&frame)?;

        match req {
            SupervisorRequest::Spawn {
                session_id,
                command,
                cwd,
                env,
                cols,
                rows,
            } => {
                let resp = match do_spawn(&session_id, &command, &cwd, &env, cols, rows) {
                    Ok((master_fd, pid)) => {
                        let child_pid = Pid::from_raw(pid as i32);
                        state.lock().unwrap().sessions.insert(
                            session_id.clone(),
                            ManagedSession {
                                session_id: session_id.clone(),
                                master_fd,
                                child_pid,
                                alive: true,
                            },
                        );
                        info!("spawned session {} pid {}", session_id, pid);
                        SupervisorResponse::Spawned { session_id, pid }
                    }
                    Err(e) => {
                        warn!("spawn failed for {}: {e}", session_id);
                        SupervisorResponse::Error {
                            message: e.to_string(),
                        }
                    }
                };
                write_frame(&mut stream, &serde_json::to_vec(&resp)?)?;
            }

            SupervisorRequest::AttachFd { session_id } => {
                let dup_result = {
                    let s = state.lock().unwrap();
                    match s.sessions.get(&session_id) {
                        None => None,
                        Some(ms) => {
                            let pid = ms.child_pid;
                            ms.master_fd.try_clone().ok().map(|fd| (fd, pid))
                        }
                    }
                };
                match dup_result {
                    None => {
                        let resp = SupervisorResponse::Error {
                            message: format!("session {session_id} not found"),
                        };
                        write_frame(&mut stream, &serde_json::to_vec(&resp)?)?;
                    }
                    Some((dup_fd, pid)) => {
                        let resp = SupervisorResponse::FdAttached {
                            session_id,
                            pid: pid.as_raw() as u32,
                        };
                        write_frame(&mut stream, &serde_json::to_vec(&resp)?)?;
                        send_fd(stream.as_raw_fd(), dup_fd.as_raw_fd())?;
                    }
                }
            }

            SupervisorRequest::Write { session_id, data } => {
                let resp = {
                    let s = state.lock().unwrap();
                    match s.sessions.get(&session_id) {
                        None => SupervisorResponse::Error {
                            message: "session not found".into(),
                        },
                        Some(ms) => match nix::unistd::write(&ms.master_fd, &data) {
                            Ok(n) => SupervisorResponse::Written { len: n },
                            Err(e) => SupervisorResponse::Error {
                                message: e.to_string(),
                            },
                        },
                    }
                };
                write_frame(&mut stream, &serde_json::to_vec(&resp)?)?;
            }

            SupervisorRequest::Resize {
                session_id,
                cols,
                rows,
            } => {
                let resp = {
                    let s = state.lock().unwrap();
                    match s.sessions.get(&session_id) {
                        None => SupervisorResponse::Error {
                            message: "session not found".into(),
                        },
                        Some(ms) => match resize_pty(ms.master_fd.as_raw_fd(), cols, rows) {
                            Ok(()) => SupervisorResponse::Resized,
                            Err(e) => SupervisorResponse::Error {
                                message: e.to_string(),
                            },
                        },
                    }
                };
                write_frame(&mut stream, &serde_json::to_vec(&resp)?)?;
            }

            SupervisorRequest::Kill { session_id, signal } => {
                let resp = {
                    let s = state.lock().unwrap();
                    match s.sessions.get(&session_id) {
                        None => SupervisorResponse::Error {
                            message: "session not found".into(),
                        },
                        Some(ms) => match Signal::try_from(signal) {
                            Ok(sig) => match nix::sys::signal::kill(ms.child_pid, sig) {
                                Ok(()) => SupervisorResponse::Killed,
                                Err(e) => SupervisorResponse::Error {
                                    message: e.to_string(),
                                },
                            },
                            Err(e) => SupervisorResponse::Error {
                                message: e.to_string(),
                            },
                        },
                    }
                };
                write_frame(&mut stream, &serde_json::to_vec(&resp)?)?;
            }

            SupervisorRequest::List => {
                let sessions = {
                    let s = state.lock().unwrap();
                    s.sessions
                        .values()
                        .map(|ms| SupervisorSessionInfo {
                            session_id: ms.session_id.clone(),
                            pid: ms.child_pid.as_raw() as u32,
                            alive: ms.alive,
                        })
                        .collect::<Vec<_>>()
                };
                let resp = SupervisorResponse::SessionList { sessions };
                write_frame(&mut stream, &serde_json::to_vec(&resp)?)?;
            }

            SupervisorRequest::Ping => {
                write_frame(&mut stream, &serde_json::to_vec(&SupervisorResponse::Pong)?)?;
            }
        }
    }
    Ok(())
}

fn do_spawn(
    _session_id: &str,
    command: &[String],
    cwd: &str,
    env: &HashMap<String, String>,
    cols: u16,
    rows: u16,
) -> Result<(OwnedFd, u32)> {
    use std::os::unix::process::CommandExt;

    if command.is_empty() {
        bail!("empty command");
    }

    let ws = nix::pty::Winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let pty = nix::pty::openpty(Some(&ws), None)?;
    let slave_raw = pty.slave.as_raw_fd();

    let mut cmd = std::process::Command::new(&command[0]);
    for arg in &command[1..] {
        cmd.arg(arg);
    }
    cmd.current_dir(cwd);
    cmd.env_clear();
    for (k, v) in env {
        cmd.env(k, v);
    }

    unsafe {
        cmd.pre_exec(move || {
            libc::setsid();
            libc::ioctl(slave_raw, libc::TIOCSCTTY.into(), 0i32);
            libc::dup2(slave_raw, 0);
            libc::dup2(slave_raw, 1);
            libc::dup2(slave_raw, 2);
            let max_fd = libc::sysconf(libc::_SC_OPEN_MAX);
            let limit = if max_fd > 0 {
                max_fd.min(1024) as i32
            } else {
                1024i32
            };
            for fd in 3..limit {
                libc::close(fd);
            }
            Ok(())
        });
    }

    let child = cmd.spawn()?;
    drop(pty.slave);

    let pid = child.id();
    drop(child);

    Ok((pty.master, pid))
}

fn resize_pty(fd: std::os::unix::io::RawFd, cols: u16, rows: u16) -> Result<()> {
    let ws = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let ret = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &ws) };
    if ret < 0 {
        bail!("TIOCSWINSZ failed: {}", std::io::Error::last_os_error());
    }
    Ok(())
}
