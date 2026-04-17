use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::drivers::{AgentDriver, PtyHandle, SpawnCfg};
use crate::events::{Event, EventBus};
use crate::ringbuf::{RingBuf, DEFAULT_CAPACITY};
use crate::session::{SessionId, SessionState};

pub struct ActiveSession {
    pub session_id: SessionId,
    pub master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    pub writer: Arc<Mutex<PtyHandle>>,
    pub output_tx: broadcast::Sender<Vec<u8>>,
    pub driver: Arc<Mutex<Box<dyn AgentDriver>>>,
    pub child: Arc<Mutex<Box<dyn Child + Send>>>,
    pub created_at: i64,
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
    let writer = pair.master.take_writer()?;

    let (output_tx, _) = broadcast::channel::<Vec<u8>>(256);

    let ring_path = ring_dir.join(session_id.as_ref()).join("output.bin");
    let mut ring_buf = RingBuf::create(&ring_path, DEFAULT_CAPACITY)?;

    let driver = Arc::new(Mutex::new(driver));
    let driver_clone = Arc::clone(&driver);
    let output_tx_clone = output_tx.clone();
    let event_bus_clone = Arc::clone(&event_bus);
    let sid = session_id.clone();

    let reader_handle = std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let chunk = buf[..n].to_vec();
                    if let Ok((offset, len)) = ring_buf.write(&chunk) {
                        event_bus_clone
                            .send(sid.to_string(), Event::OutputAppended { offset, len });
                    }
                    let events = driver_clone.lock().unwrap().on_bytes(&chunk);
                    for evt in events {
                        event_bus_clone.send(
                            sid.to_string(),
                            Event::AgentEvent {
                                id: sid.clone(),
                                event: evt,
                            },
                        );
                    }
                    let _ = output_tx_clone.send(chunk);
                }
            }
        }
        let _ = ring_buf.sync();
    });

    let cleanup_sid = session_id.clone();
    let cleanup_bus = Arc::clone(&event_bus);
    tokio::spawn(async move {
        let _ = tokio::task::spawn_blocking(move || reader_handle.join()).await;
        let _ =
            crate::session::Session::update_state(&db, &cleanup_sid, SessionState::Exited).await;
        cleanup_bus.send(
            cleanup_sid.to_string(),
            Event::StateChanged {
                from: SessionState::Idle,
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
        master: Arc::new(Mutex::new(pair.master)),
        writer: Arc::new(Mutex::new(PtyHandle::new(writer))),
        output_tx,
        driver,
        child: Arc::new(Mutex::new(child)),
        created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::drivers::shell::ShellDriver;
    use crate::drivers::SpawnRequest;
    use crate::session::SessionKind;
    use std::collections::HashMap;

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

        // Write a command
        active
            .writer
            .lock()
            .unwrap()
            .write_all(b"echo hello\r")
            .unwrap();

        // Read with timeout
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

        // Clean exit
        active.writer.lock().unwrap().write_all(b"exit\r").unwrap();
    }
}
