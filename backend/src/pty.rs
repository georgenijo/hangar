use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use sqlx::SqlitePool;
use std::os::unix::io::OwnedFd;
use tokio::sync::broadcast;

use crate::drivers::{AgentDriver, SpawnCfg, StateCtx};
use crate::events::{AgentEvent, Event, EventBus};
use crate::raw_fd_master::RawFdMaster;
use crate::ringbuf::{RingBuf, DEFAULT_CAPACITY};
use crate::sandbox::{SandboxState, SandboxStatus};
use crate::session::{AgentMeta, Session, SessionId, SessionState};
use crate::util;

/// Reap a PTY child process to prevent it lingering as a zombie.
///
/// `std::process::Child` (what portable_pty returns on Unix) does NOT call
/// `waitpid` on drop, so hangard's `DELETE /sessions/:id` path used to leave
/// `Zs` entries parented to hangard until hangard itself exited. See #51.
///
/// Strategy: poll `try_wait` during a grace window so cleanly-exiting
/// children (driver.shutdown already sent ctrl-d / `/exit`) are reaped
/// without an extra signal; if still alive at the deadline, `kill()` (which
/// portable_pty escalates SIGHUP → SIGKILL internally) and then block on
/// `wait()`.
pub fn reap_child(child_arc: Arc<Mutex<Box<dyn Child + Send>>>, grace: Duration) {
    let mut child = match child_arc.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    let deadline = std::time::Instant::now() + grace;
    while std::time::Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => break,
        }
    }

    let _ = child.kill();
    let _ = child.wait();
}

pub enum PtyMaster {
    Local(Arc<Mutex<Box<dyn MasterPty + Send>>>),
    Attached(Arc<Mutex<RawFdMaster>>),
}

impl PtyMaster {
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        match self {
            PtyMaster::Local(m) => {
                m.lock()
                    .unwrap()
                    .resize(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    })
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                Ok(())
            }
            PtyMaster::Attached(m) => m.lock().unwrap().resize(cols, rows),
        }
    }
}

impl Clone for PtyMaster {
    fn clone(&self) -> Self {
        match self {
            PtyMaster::Local(a) => PtyMaster::Local(Arc::clone(a)),
            PtyMaster::Attached(a) => PtyMaster::Attached(Arc::clone(a)),
        }
    }
}

pub struct ActiveSession {
    pub session_id: SessionId,
    pub master: PtyMaster,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub output_tx: broadcast::Sender<Vec<u8>>,
    pub driver: Arc<Mutex<Box<dyn AgentDriver>>>,
    pub child: Option<Arc<Mutex<Box<dyn Child + Send>>>>,
    pub created_at: i64,
}

pub struct ReaderLoopCtx {
    pub session_id: SessionId,
    pub driver: Arc<Mutex<Box<dyn AgentDriver>>>,
    pub event_bus: Arc<EventBus>,
    pub output_tx: broadcast::Sender<Vec<u8>>,
    pub ring_buf: RingBuf,
    pub last_bytes_ms: Arc<AtomicI64>,
    pub current_state: Arc<Mutex<SessionState>>,
}

const MAX_OUTPUT_INDEX_BYTES: usize = 4096;

pub fn indexable_text_from_chunk(chunk: &[u8]) -> Option<String> {
    let lossy = String::from_utf8_lossy(chunk);
    let stripped = util::strip_ansi(&lossy);
    let truncated: String = stripped
        .chars()
        .scan(0usize, |n, c| {
            *n += c.len_utf8();
            if *n > MAX_OUTPUT_INDEX_BYTES {
                None
            } else {
                Some(c)
            }
        })
        .collect();
    if truncated.trim().is_empty() {
        None
    } else {
        Some(truncated)
    }
}

fn run_reader_loop(
    mut reader: Box<dyn Read + Send>,
    mut ctx: ReaderLoopCtx,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    ctx.last_bytes_ms
                        .store(crate::util::now_ms() as i64, Ordering::Relaxed);
                    let chunk = buf[..n].to_vec();
                    if let Ok((offset, len)) = ctx.ring_buf.write(&chunk) {
                        let text = indexable_text_from_chunk(&chunk);
                        ctx.event_bus.send(
                            ctx.session_id.to_string(),
                            Event::OutputAppended { offset, len, text },
                        );
                    }
                    let events = ctx.driver.lock().unwrap().on_bytes(&chunk);
                    for evt in events {
                        ctx.event_bus.send(
                            ctx.session_id.to_string(),
                            Event::AgentEvent {
                                id: ctx.session_id.clone(),
                                event: evt,
                            },
                        );
                    }
                    let _ = ctx.output_tx.send(chunk);
                }
            }
        }
        let _ = ctx.ring_buf.sync();
    })
}

fn spawn_watchdog(
    session_id: SessionId,
    driver: Arc<Mutex<Box<dyn AgentDriver>>>,
    event_bus: Arc<EventBus>,
    last_bytes_ms: Arc<AtomicI64>,
    current_state: Arc<Mutex<SessionState>>,
    db_pool: SqlitePool,
    mut event_rx: broadcast::Receiver<(String, Event)>,
) -> std::thread::JoinHandle<()> {
    let handle = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let mut last_event: Option<AgentEvent> = None;
        let mut event_timestamps: VecDeque<i64> = VecDeque::new();
        let mut last_activity_ms: i64 = 0;
        let sid_str = session_id.to_string();
        let mut agent_meta: Option<AgentMeta> = None;
        let mut meta_dirty = false;

        loop {
            std::thread::sleep(Duration::from_secs(5));

            // Drain pending events from bus
            loop {
                match event_rx.try_recv() {
                    Ok((sid, Event::AgentEvent { event, .. })) if sid == sid_str => {
                        let ts = crate::util::now_ms() as i64;
                        event_timestamps.push_back(ts);
                        if event_timestamps.len() > 20 {
                            event_timestamps.pop_front();
                        }
                        last_activity_ms = ts;

                        let meta = agent_meta.get_or_insert_with(|| AgentMeta {
                            name: "claude_code".to_string(),
                            version: None,
                            model: None,
                            tokens_used: 0,
                            last_tool_call: None,
                            context_pct: None,
                            cost_dollars: None,
                        });

                        match &event {
                            AgentEvent::ModelChanged { model } => {
                                meta.model = Some(model.clone());
                                meta_dirty = true;
                            }
                            AgentEvent::TurnFinished { tokens_used, .. } => {
                                meta.tokens_used += *tokens_used as u64;
                                meta_dirty = true;
                            }
                            AgentEvent::ToolCallStarted { tool, .. } => {
                                meta.last_tool_call = Some(tool.clone());
                                meta_dirty = true;
                            }
                            AgentEvent::ContextWindowSizeChanged { pct_used, .. } => {
                                meta.context_pct = Some(*pct_used);
                                meta_dirty = true;
                            }
                            AgentEvent::CostUpdated { dollars } => {
                                meta.cost_dollars = Some(*dollars);
                                meta_dirty = true;
                            }
                            _ => {}
                        }

                        last_event = Some(event);
                    }
                    Ok(_) => {}
                    Err(broadcast::error::TryRecvError::Empty) => break,
                    Err(broadcast::error::TryRecvError::Lagged(n)) => {
                        tracing::warn!(
                            session_id = %sid_str,
                            dropped = n,
                            "watchdog lagged: agent_meta may be stale for dropped events",
                        );
                    }
                    Err(broadcast::error::TryRecvError::Closed) => return,
                }
            }

            if meta_dirty {
                if let Some(ref meta) = agent_meta {
                    let _ = handle.block_on(Session::update_agent_meta(
                        &db_pool,
                        &session_id,
                        meta,
                    ));
                }
                meta_dirty = false;
            }

            let current = current_state.lock().unwrap().clone();
            if current == SessionState::Exited {
                break;
            }

            let ctx = StateCtx {
                current_state: current.clone(),
                last_activity_ms,
                last_event: last_event.clone(),
                last_bytes_ms: last_bytes_ms.load(Ordering::Relaxed),
                event_timestamps: event_timestamps.iter().copied().collect(),
            };

            if let Some(new_state) = driver.lock().unwrap().detect_state(&ctx) {
                if new_state != current {
                    let old_state = current.clone();
                    *current_state.lock().unwrap() = new_state.clone();
                    let _ = handle.block_on(crate::session::Session::update_state(
                        &db_pool,
                        &session_id,
                        new_state.clone(),
                    ));
                    event_bus.send(
                        sid_str.clone(),
                        Event::StateChanged {
                            from: old_state,
                            to: new_state,
                        },
                    );
                }
            }
        }
    })
}

pub fn spawn_pty(
    session_id: SessionId,
    spawn_cfg: SpawnCfg,
    driver: Box<dyn AgentDriver>,
    ring_dir: PathBuf,
    event_bus: Arc<EventBus>,
    db: SqlitePool,
    initial_size: (u16, u16),
) -> Result<ActiveSession> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: initial_size.1,
        cols: initial_size.0,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(&spawn_cfg.command[0]);
    for arg in &spawn_cfg.command[1..] {
        cmd.arg(arg);
    }
    cmd.cwd(&spawn_cfg.cwd);
    for (k, v) in &spawn_cfg.env {
        cmd.env(k, v);
    }

    let child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let reader = pair.master.try_clone_reader()?;
    let writer: Box<dyn Write + Send> = pair.master.take_writer()?;

    let (output_tx, _) = broadcast::channel::<Vec<u8>>(256);

    let ring_path = ring_dir.join(session_id.as_ref()).join("output.bin");
    let ring_buf = RingBuf::create(&ring_path, DEFAULT_CAPACITY)?;

    let last_bytes_ms = Arc::new(AtomicI64::new(0));
    let current_state = Arc::new(Mutex::new(SessionState::Booting));

    let driver = Arc::new(Mutex::new(driver));

    let reader_ctx = ReaderLoopCtx {
        session_id: session_id.clone(),
        driver: Arc::clone(&driver),
        event_bus: Arc::clone(&event_bus),
        output_tx: output_tx.clone(),
        ring_buf,
        last_bytes_ms: Arc::clone(&last_bytes_ms),
        current_state: Arc::clone(&current_state),
    };

    let reader_handle = run_reader_loop(reader, reader_ctx);

    let event_rx = event_bus.subscribe();
    let _watchdog = spawn_watchdog(
        session_id.clone(),
        Arc::clone(&driver),
        Arc::clone(&event_bus),
        Arc::clone(&last_bytes_ms),
        Arc::clone(&current_state),
        db.clone(),
        event_rx,
    );

    let cleanup_sid = session_id.clone();
    let cleanup_bus = Arc::clone(&event_bus);
    let current_state_cleanup = Arc::clone(&current_state);
    tokio::spawn(async move {
        let _ = tokio::task::spawn_blocking(move || reader_handle.join()).await;
        let from = current_state_cleanup.lock().unwrap().clone();
        *current_state_cleanup.lock().unwrap() = SessionState::Exited;
        let _ =
            crate::session::Session::update_state(&db, &cleanup_sid, SessionState::Exited).await;
        cleanup_bus.send(
            cleanup_sid.to_string(),
            Event::StateChanged {
                from,
                to: SessionState::Exited,
            },
        );
        // If session had a running sandbox, mark it stopped
        if let Ok(Some(session)) = Session::get(&db, &cleanup_sid).await {
            if let Some(sb) = session.sandbox {
                if matches!(sb.state, SandboxState::Running | SandboxState::Creating) {
                    let _ = Session::update_sandbox_state(
                        &db,
                        cleanup_sid.as_ref(),
                        SandboxState::Stopped,
                    )
                    .await;
                    cleanup_bus.send(
                        cleanup_sid.to_string(),
                        Event::AgentEvent {
                            id: cleanup_sid.clone(),
                            event: AgentEvent::SandboxStateChanged {
                                state: SandboxState::Stopped,
                            },
                        },
                    );
                }
            }
        }
    });

    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    Ok(ActiveSession {
        session_id,
        master: PtyMaster::Local(Arc::new(Mutex::new(pair.master))),
        writer: Arc::new(Mutex::new(writer)),
        output_tx,
        driver,
        child: Some(Arc::new(Mutex::new(child))),
        created_at,
    })
}

/// Wraps an existing command vector with `sudo podman exec` to run inside a
/// pre-created sandbox container.
///
/// # Known limitation
/// Without `-t`, curses apps (vim, htop) won't receive SIGWINCH inside the
/// container. Set `allocate_tty: true` in SandboxSpec for interactive shell
/// sessions. This may cause double-PTY signal issues — monitor and report.
#[allow(clippy::too_many_arguments)]
pub async fn spawn_pty_sandboxed(
    session_id: SessionId,
    spawn_cfg: SpawnCfg,
    sandbox_status: &SandboxStatus,
    driver: Box<dyn AgentDriver>,
    ring_dir: PathBuf,
    event_bus: Arc<EventBus>,
    db: SqlitePool,
    initial_size: (u16, u16),
) -> Result<ActiveSession> {
    let tty_flag = if sandbox_status.spec.allocate_tty {
        "-t"
    } else {
        "-i"
    };

    let container_name = &sandbox_status.container_name;
    let mut wrapped = vec![
        "sudo".to_string(),
        "podman".to_string(),
        "exec".to_string(),
        tty_flag.to_string(),
        container_name.clone(),
        "--".to_string(),
    ];
    wrapped.extend(spawn_cfg.command.iter().cloned());

    let wrapped_cfg = SpawnCfg {
        command: wrapped,
        env: spawn_cfg.env,
        cwd: spawn_cfg.cwd,
        temp_files: spawn_cfg.temp_files,
    };

    spawn_pty(
        session_id,
        wrapped_cfg,
        driver,
        ring_dir,
        event_bus,
        db,
        initial_size,
    )
}

pub fn reattach(
    session_id: SessionId,
    master_fd: OwnedFd,
    driver: Box<dyn AgentDriver>,
    ring_dir: PathBuf,
    event_bus: Arc<EventBus>,
    db: SqlitePool,
) -> Result<ActiveSession> {
    let ring_path = ring_dir.join(session_id.as_ref()).join("output.bin");
    let ring_buf = if ring_path.exists() {
        RingBuf::open(&ring_path)?
    } else {
        if let Some(parent) = ring_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        RingBuf::create(&ring_path, DEFAULT_CAPACITY)?
    };

    let read_fd = master_fd.try_clone()?;
    let write_fd = master_fd.try_clone()?;
    let resize_fd = master_fd;

    let (output_tx, _) = broadcast::channel::<Vec<u8>>(256);

    let last_bytes_ms = Arc::new(AtomicI64::new(0));
    let current_state = Arc::new(Mutex::new(SessionState::Booting));

    let driver = Arc::new(Mutex::new(driver));

    let reader_ctx = ReaderLoopCtx {
        session_id: session_id.clone(),
        driver: Arc::clone(&driver),
        event_bus: Arc::clone(&event_bus),
        output_tx: output_tx.clone(),
        ring_buf,
        last_bytes_ms: Arc::clone(&last_bytes_ms),
        current_state: Arc::clone(&current_state),
    };

    let reader = Box::new(RawFdMaster::new(read_fd));
    let reader_handle = run_reader_loop(reader, reader_ctx);

    let event_rx = event_bus.subscribe();
    let _watchdog = spawn_watchdog(
        session_id.clone(),
        Arc::clone(&driver),
        Arc::clone(&event_bus),
        Arc::clone(&last_bytes_ms),
        Arc::clone(&current_state),
        db.clone(),
        event_rx,
    );

    let cleanup_sid = session_id.clone();
    let cleanup_bus = Arc::clone(&event_bus);
    let current_state_cleanup = Arc::clone(&current_state);
    tokio::spawn(async move {
        let _ = tokio::task::spawn_blocking(move || reader_handle.join()).await;
        let from = current_state_cleanup.lock().unwrap().clone();
        *current_state_cleanup.lock().unwrap() = SessionState::Exited;
        let _ =
            crate::session::Session::update_state(&db, &cleanup_sid, SessionState::Exited).await;
        cleanup_bus.send(
            cleanup_sid.to_string(),
            Event::StateChanged {
                from,
                to: SessionState::Exited,
            },
        );
    });

    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    Ok(ActiveSession {
        session_id,
        master: PtyMaster::Attached(Arc::new(Mutex::new(RawFdMaster::new(resize_fd)))),
        writer: Arc::new(Mutex::new(Box::new(RawFdMaster::new(write_fd)))),
        output_tx,
        driver,
        child: None,
        created_at,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn spawn_pty_supervised(
    supervisor: &crate::supervisor_client::SupervisorClient,
    session_id: SessionId,
    spawn_cfg: SpawnCfg,
    driver: Box<dyn AgentDriver>,
    ring_dir: PathBuf,
    event_bus: Arc<EventBus>,
    db: SqlitePool,
    initial_size: (u16, u16),
) -> Result<ActiveSession> {
    let cwd = spawn_cfg.cwd.display().to_string();
    supervisor
        .spawn_session(
            session_id.to_string(),
            spawn_cfg.command.clone(),
            cwd,
            spawn_cfg.env.clone(),
            initial_size.0,
            initial_size.1,
        )
        .await?;

    let (_, fd) = supervisor.attach_fd(session_id.as_ref()).await?;

    reattach(session_id, fd, driver, ring_dir, event_bus, db)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::drivers::shell::ShellDriver;
    use crate::drivers::SpawnRequest;
    use crate::session::SessionKind;
    use std::collections::HashMap;

    #[test]
    fn sandboxed_command_wrapping_no_tty() {
        use crate::sandbox::{SandboxSpec, SandboxState, SandboxStatus};
        use std::path::PathBuf;

        let status = SandboxStatus {
            spec: SandboxSpec {
                allocate_tty: false,
                ..SandboxSpec::default()
            },
            state: SandboxState::Running,
            container_name: "hangar-ABC".to_string(),
            overlay_dir: PathBuf::from("/tmp/o"),
            project_dir: PathBuf::from("/home/user/project"),
            merged_dir: PathBuf::from("/tmp/o/merged"),
        };

        let inner_cmd = ["claude", "--arg"];
        let tty_flag = if status.spec.allocate_tty { "-t" } else { "-i" };
        let mut wrapped: Vec<String> = [
            "sudo",
            "podman",
            "exec",
            tty_flag,
            &status.container_name,
            "--",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        wrapped.extend(inner_cmd.iter().map(|s| s.to_string()));

        assert_eq!(wrapped[0], "sudo");
        assert_eq!(wrapped[1], "podman");
        assert_eq!(wrapped[2], "exec");
        assert_eq!(wrapped[3], "-i");
        assert_eq!(wrapped[4], "hangar-ABC");
        assert_eq!(wrapped[5], "--");
        assert_eq!(wrapped[6], "claude");
        assert_eq!(wrapped[7], "--arg");
    }

    #[test]
    fn sandboxed_command_wrapping_with_tty() {
        use crate::sandbox::{SandboxSpec, SandboxState, SandboxStatus};
        use std::path::PathBuf;

        let status = SandboxStatus {
            spec: SandboxSpec {
                allocate_tty: true,
                ..SandboxSpec::default()
            },
            state: SandboxState::Running,
            container_name: "hangar-XYZ".to_string(),
            overlay_dir: PathBuf::from("/tmp/o"),
            project_dir: PathBuf::from("/home/user/project"),
            merged_dir: PathBuf::from("/tmp/o/merged"),
        };

        let tty_flag = if status.spec.allocate_tty { "-t" } else { "-i" };
        let wrapped: Vec<String> = [
            "sudo",
            "podman",
            "exec",
            tty_flag,
            &status.container_name,
            "--",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        assert_eq!(wrapped[3], "-t");
    }

    #[tokio::test]
    #[ignore = "requires PTY allocation; run manually"]
    async fn test_spawn_and_read_output() {
        let db = Db::new_in_memory().await.unwrap();
        let event_bus = Arc::new(EventBus::new());
        let ring_dir = tempfile::tempdir().unwrap();

        let session_id = SessionId::new();
        let driver = Box::new(ShellDriver::new());

        let spawn_req = SpawnRequest {
            session_id: session_id.clone(),
            cwd: std::env::current_dir().unwrap(),
            env: HashMap::new(),
            kind: SessionKind::Shell,
            hmac_key: vec![],
        };
        let spawn_cfg = driver.spawn_cfg(&spawn_req).unwrap();

        let active = spawn_pty(
            session_id.clone(),
            spawn_cfg,
            driver,
            ring_dir.path().to_path_buf(),
            event_bus,
            db.pool().clone(),
            (80, 24),
        )
        .unwrap();

        let mut rx = active.output_tx.subscribe();

        active
            .writer
            .lock()
            .unwrap()
            .write_all(b"echo hello\r")
            .unwrap();

        let result = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
            let mut collected = Vec::new();
            loop {
                match rx.recv().await {
                    Ok(chunk) => {
                        collected.extend_from_slice(&chunk);
                        let s = String::from_utf8_lossy(&collected);
                        if s.contains("hello") {
                            return true;
                        }
                    }
                    Err(_) => return false,
                }
            }
        })
        .await;

        assert!(result.unwrap_or(false), "expected 'hello' in PTY output");

        active.writer.lock().unwrap().write_all(b"exit\r").unwrap();
    }
}
