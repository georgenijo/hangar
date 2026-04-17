use std::collections::HashMap;
use std::os::unix::io::{AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{bail, Result};

use crate::supervisor_protocol::{
    read_frame, recv_fd, write_frame, SupervisorRequest, SupervisorResponse, SupervisorSessionInfo,
};

struct Inner {
    stream: Mutex<UnixStream>,
    connected: AtomicBool,
}

#[derive(Clone)]
pub struct SupervisorClient {
    inner: Arc<Inner>,
}

impl SupervisorClient {
    pub fn connect(path: &Path) -> Result<Self> {
        let mut last_err = anyhow::anyhow!("no attempts");
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            match UnixStream::connect(path) {
                Ok(stream) => {
                    return Ok(SupervisorClient {
                        inner: Arc::new(Inner {
                            stream: Mutex::new(stream),
                            connected: AtomicBool::new(true),
                        }),
                    });
                }
                Err(e) => {
                    last_err = e.into();
                }
            }
        }
        Err(last_err)
    }

    pub fn is_connected(&self) -> bool {
        self.inner.connected.load(Ordering::Relaxed)
    }

    fn send_request(inner: &Inner, req: &SupervisorRequest) -> Result<SupervisorResponse> {
        let mut stream = inner.stream.lock().unwrap();
        let bytes = serde_json::to_vec(req)?;
        write_frame(&mut *stream, &bytes)?;
        let frame = read_frame(&mut *stream)?;
        let resp: SupervisorResponse = serde_json::from_slice(&frame)?;
        Ok(resp)
    }

    fn send_request_with_fd(
        inner: &Inner,
        req: &SupervisorRequest,
    ) -> Result<(SupervisorResponse, OwnedFd)> {
        let sock_raw = {
            let stream = inner.stream.lock().unwrap();
            stream.as_raw_fd()
        };
        let resp = Self::send_request(inner, req)?;
        let fd = recv_fd(sock_raw)?;
        Ok((resp, fd))
    }

    pub async fn spawn_session(
        &self,
        session_id: String,
        command: Vec<String>,
        cwd: String,
        env: HashMap<String, String>,
        cols: u16,
        rows: u16,
    ) -> Result<u32> {
        let inner = Arc::clone(&self.inner);
        let req = SupervisorRequest::Spawn {
            session_id,
            command,
            cwd,
            env,
            cols,
            rows,
        };
        let resp = tokio::task::spawn_blocking(move || Self::send_request(&inner, &req)).await??;
        match resp {
            SupervisorResponse::Spawned { pid, .. } => Ok(pid),
            SupervisorResponse::Error { message } => bail!("supervisor: {}", message),
            _ => bail!("unexpected supervisor response to Spawn"),
        }
    }

    pub async fn attach_fd(&self, session_id: &str) -> Result<(SupervisorResponse, OwnedFd)> {
        let inner = Arc::clone(&self.inner);
        let req = SupervisorRequest::AttachFd {
            session_id: session_id.to_string(),
        };
        tokio::task::spawn_blocking(move || Self::send_request_with_fd(&inner, &req)).await?
    }

    pub async fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let req = SupervisorRequest::Resize {
            session_id: session_id.to_string(),
            cols,
            rows,
        };
        let resp = tokio::task::spawn_blocking(move || Self::send_request(&inner, &req)).await??;
        match resp {
            SupervisorResponse::Resized => Ok(()),
            SupervisorResponse::Error { message } => bail!("supervisor: {}", message),
            _ => bail!("unexpected supervisor response to Resize"),
        }
    }

    pub async fn kill(&self, session_id: &str, signal: i32) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let req = SupervisorRequest::Kill {
            session_id: session_id.to_string(),
            signal,
        };
        let resp = tokio::task::spawn_blocking(move || Self::send_request(&inner, &req)).await??;
        match resp {
            SupervisorResponse::Killed => Ok(()),
            SupervisorResponse::Error { message } => bail!("supervisor: {}", message),
            _ => bail!("unexpected supervisor response to Kill"),
        }
    }

    pub async fn list(&self) -> Result<Vec<SupervisorSessionInfo>> {
        let inner = Arc::clone(&self.inner);
        let req = SupervisorRequest::List;
        let resp = tokio::task::spawn_blocking(move || Self::send_request(&inner, &req)).await??;
        match resp {
            SupervisorResponse::SessionList { sessions } => Ok(sessions),
            SupervisorResponse::Error { message } => bail!("supervisor: {}", message),
            _ => bail!("unexpected supervisor response to List"),
        }
    }

    pub async fn ping(&self) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let req = SupervisorRequest::Ping;
        let resp = tokio::task::spawn_blocking(move || Self::send_request(&inner, &req)).await??;
        match resp {
            SupervisorResponse::Pong => Ok(()),
            _ => bail!("unexpected supervisor response to Ping"),
        }
    }
}
