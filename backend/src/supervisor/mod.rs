pub mod child;
pub mod fd_pass;
pub mod protocol;

use std::os::unix::io::{OwnedFd, RawFd};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::Result;
use tokio::net::UnixStream;
use uuid::Uuid;

use protocol::*;

pub struct SupervisorHandle {
    cmd_stream: tokio::sync::Mutex<UnixStream>,
    fd_stream_fd: RawFd,
    // Keep OwnedFd alive for fd_stream lifetime
    _fd_stream_owned: OwnedFd,
    nonce_counter: AtomicU64,
    pub connected: Arc<AtomicBool>,
}

impl SupervisorHandle {
    pub async fn connect(socket_path: &Path) -> Result<Arc<Self>> {
        let client_id = Uuid::new_v4();

        // Primary connection
        let mut primary = UnixStream::connect(socket_path).await?;
        let handshake = Handshake {
            client_id,
            role: HandshakeRole::Primary,
        };
        write_frame(&mut primary, &serde_json::to_vec(&handshake)?).await?;
        // Read ack
        let _ack = read_frame(&mut primary).await?;

        // FdChannel connection (sync, for sendmsg/recvmsg)
        let fd_channel = std::os::unix::net::UnixStream::connect(socket_path)?;
        let handshake2 = Handshake {
            client_id,
            role: HandshakeRole::FdChannel,
        };
        let data = serde_json::to_vec(&handshake2)?;
        let len = (data.len() as u32).to_be_bytes();
        use std::io::{Read, Write};
        let mut fd_channel_clone = fd_channel.try_clone()?;
        fd_channel_clone.write_all(&len)?;
        fd_channel_clone.write_all(&data)?;
        // Read ack
        let mut ack_len = [0u8; 4];
        fd_channel_clone.read_exact(&mut ack_len)?;
        let ack_n = u32::from_be_bytes(ack_len) as usize;
        let mut ack_buf = vec![0u8; ack_n];
        fd_channel_clone.read_exact(&mut ack_buf)?;

        // Convert to OwnedFd — we keep fd_channel alive via _fd_stream_owned
        use std::os::unix::io::{FromRawFd, IntoRawFd};
        let raw_fd = fd_channel.into_raw_fd();
        let fd_stream_fd = raw_fd;
        let owned_fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

        let connected = Arc::new(AtomicBool::new(true));
        Ok(Arc::new(Self {
            cmd_stream: tokio::sync::Mutex::new(primary),
            fd_stream_fd,
            _fd_stream_owned: owned_fd,
            nonce_counter: AtomicU64::new(0),
            connected,
        }))
    }

    async fn send_cmd(&self, cmd: &SupervisorCmd) -> Result<SupervisorResp> {
        let data = serde_json::to_vec(cmd)?;
        let mut stream = self.cmd_stream.lock().await;
        write_frame(&mut *stream, &data).await?;
        let resp_data = read_frame(&mut *stream).await?;
        Ok(serde_json::from_slice(&resp_data)?)
    }

    pub async fn spawn(&self, cmd: SupervisorCmd) -> Result<SupervisorResp> {
        self.send_cmd(&cmd).await
    }

    pub async fn attach_fd(&self, id: &str) -> Result<OwnedFd> {
        let nonce = self.nonce_counter.fetch_add(1, Ordering::SeqCst);
        let cmd = SupervisorCmd::AttachFd {
            id: id.to_string(),
            nonce,
        };
        // Send on cmd_stream
        {
            let data = serde_json::to_vec(&cmd)?;
            let mut stream = self.cmd_stream.lock().await;
            write_frame(&mut *stream, &data).await?;
            // Don't read response here — it comes with the fd on fd_channel
        }
        // Receive fd on fd_channel
        let mut payload_buf = vec![0u8; 256];
        let (n, fd_opt) = fd_pass::recv_fd(self.fd_stream_fd, &mut payload_buf)?;
        let fd = fd_opt.ok_or_else(|| anyhow::anyhow!("no fd received from supervisor"))?;
        // Verify nonce
        let payload = &payload_buf[..n];
        let val: serde_json::Value = serde_json::from_slice(payload)?;
        let recv_nonce = val["nonce"].as_u64().unwrap_or(u64::MAX);
        if recv_nonce != nonce {
            anyhow::bail!("nonce mismatch: expected {}, got {}", nonce, recv_nonce);
        }
        Ok(fd)
    }

    pub async fn write_to(&self, id: &str, data: &[u8]) -> Result<()> {
        let cmd = SupervisorCmd::Write {
            id: id.to_string(),
            data: Base64Bytes::from(data),
        };
        match self.send_cmd(&cmd).await? {
            SupervisorResp::Written => Ok(()),
            SupervisorResp::Error { message } => anyhow::bail!("{}", message),
            r => anyhow::bail!("unexpected response: {:?}", r),
        }
    }

    pub async fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        let cmd = SupervisorCmd::Resize {
            id: id.to_string(),
            cols,
            rows,
        };
        match self.send_cmd(&cmd).await? {
            SupervisorResp::Resized => Ok(()),
            SupervisorResp::Error { message } => anyhow::bail!("{}", message),
            r => anyhow::bail!("unexpected response: {:?}", r),
        }
    }

    pub async fn kill_session(&self, id: &str, signal: i32) -> Result<()> {
        let cmd = SupervisorCmd::Kill {
            id: id.to_string(),
            signal,
        };
        match self.send_cmd(&cmd).await? {
            SupervisorResp::Killed => Ok(()),
            SupervisorResp::Error { message } => anyhow::bail!("{}", message),
            r => anyhow::bail!("unexpected response: {:?}", r),
        }
    }

    pub async fn list(&self) -> Result<Vec<SessionInfo>> {
        match self.send_cmd(&SupervisorCmd::List).await? {
            SupervisorResp::Sessions(s) => Ok(s),
            SupervisorResp::Error { message } => anyhow::bail!("{}", message),
            r => anyhow::bail!("unexpected response: {:?}", r),
        }
    }

    pub async fn register_fd(&self, id: &str, pid: u32, fd: RawFd) -> Result<()> {
        // Send fd via fd_channel, then send RegisterFd command
        let nonce = self.nonce_counter.fetch_add(1, Ordering::SeqCst);
        let payload = serde_json::to_vec(&serde_json::json!({"nonce": nonce}))?;
        fd_pass::send_fd(self.fd_stream_fd, fd, &payload)?;
        let cmd = SupervisorCmd::RegisterFd {
            id: id.to_string(),
            pid,
        };
        match self.send_cmd(&cmd).await? {
            SupervisorResp::Spawned { .. } => Ok(()),
            SupervisorResp::Error { message } => anyhow::bail!("{}", message),
            r => anyhow::bail!("unexpected response: {:?}", r),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
}
