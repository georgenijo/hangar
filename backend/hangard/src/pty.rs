use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

pub type SpawnedPty = (Box<dyn MasterPty + Send>, Box<dyn Child + Send>);

#[derive(Debug)]
pub enum PtyError {
    Pty(Box<dyn std::error::Error + Send + Sync>),
    Io(std::io::Error),
}

impl std::fmt::Display for PtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PtyError::Pty(e) => write!(f, "pty error: {e}"),
            PtyError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl From<std::io::Error> for PtyError {
    fn from(e: std::io::Error) -> Self {
        PtyError::Io(e)
    }
}

pub fn spawn_pty(
    command: &[String],
    cwd: &Path,
    env: &BTreeMap<String, String>,
    cols: u16,
    rows: u16,
) -> Result<SpawnedPty, PtyError> {
    let pty_system = native_pty_system();
    let size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };
    let pair = pty_system
        .openpty(size)
        .map_err(|e| PtyError::Pty(e.into()))?;

    let mut cmd = CommandBuilder::new(&command[0]);
    for arg in command.iter().skip(1) {
        cmd.arg(arg);
    }
    cmd.cwd(cwd);
    for (k, v) in env {
        cmd.env(k, v);
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| PtyError::Pty(e.into()))?;

    Ok((pair.master, child))
}

pub fn start_reader_loop(master: &dyn MasterPty, tx: broadcast::Sender<Vec<u8>>) -> JoinHandle<()> {
    let mut reader = master.try_clone_reader().expect("clone pty reader");
    tokio::task::spawn_blocking(move || {
        let mut buf = vec![0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = tx.send(buf[..n].to_vec());
                }
                Err(_) => break,
            }
        }
    })
}

pub fn resize_pty(master: &dyn MasterPty, cols: u16, rows: u16) -> Result<(), PtyError> {
    master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| PtyError::Pty(e.into()))
}
