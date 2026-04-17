use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, OwnedFd, RawFd};
use std::path::PathBuf;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SupervisorRequest {
    Spawn {
        session_id: String,
        command: Vec<String>,
        cwd: String,
        env: HashMap<String, String>,
        cols: u16,
        rows: u16,
    },
    AttachFd {
        session_id: String,
    },
    Write {
        session_id: String,
        data: Vec<u8>,
    },
    Resize {
        session_id: String,
        cols: u16,
        rows: u16,
    },
    Kill {
        session_id: String,
        signal: i32,
    },
    List,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SupervisorResponse {
    Spawned {
        session_id: String,
        pid: u32,
    },
    FdAttached {
        session_id: String,
        pid: u32,
    },
    Written {
        len: usize,
    },
    Resized,
    Killed,
    SessionList {
        sessions: Vec<SupervisorSessionInfo>,
    },
    Pong,
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorSessionInfo {
    pub session_id: String,
    pub pid: u32,
    pub alive: bool,
}

pub fn supervisor_sock_path() -> PathBuf {
    dirs::state_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("hangar/supervisor.sock")
}

pub fn write_frame(stream: &mut impl Write, data: &[u8]) -> Result<()> {
    let len = data.len() as u32;
    stream.write_all(&len.to_le_bytes())?;
    stream.write_all(data)?;
    Ok(())
}

pub fn read_frame(stream: &mut impl Read) -> Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn send_fd(sock: RawFd, fd: RawFd) -> Result<()> {
    use nix::sys::socket::{sendmsg, ControlMessage, MsgFlags, UnixAddr};
    use std::io::IoSlice;

    let fds = [fd];
    let cmsg = [ControlMessage::ScmRights(&fds)];
    let iov = [IoSlice::new(b"\x00")];
    sendmsg::<UnixAddr>(sock, &iov, &cmsg, MsgFlags::empty(), None)?;
    Ok(())
}

pub fn recv_fd(sock: RawFd) -> Result<OwnedFd> {
    use nix::sys::socket::{recvmsg, ControlMessageOwned, MsgFlags, UnixAddr};
    use std::io::IoSliceMut;

    let mut buf = [0u8; 1];
    let mut iov = [IoSliceMut::new(&mut buf)];
    let mut cmsg_space = nix::cmsg_space!([RawFd; 1]);
    let msg = recvmsg::<UnixAddr>(sock, &mut iov, Some(&mut cmsg_space), MsgFlags::empty())?;
    for cmsg in msg.cmsgs()? {
        if let ControlMessageOwned::ScmRights(fds) = cmsg {
            if let Some(&raw) = fds.first() {
                return Ok(unsafe { OwnedFd::from_raw_fd(raw) });
            }
        }
    }
    bail!("no fd received in SCM_RIGHTS message")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_spawn_request() {
        let req = SupervisorRequest::Spawn {
            session_id: "test-id".to_string(),
            command: vec!["bash".to_string()],
            cwd: "/tmp".to_string(),
            env: HashMap::new(),
            cols: 80,
            rows: 24,
        };
        let bytes = serde_json::to_vec(&req).unwrap();
        let decoded: SupervisorRequest = serde_json::from_slice(&bytes).unwrap();
        match decoded {
            SupervisorRequest::Spawn {
                session_id,
                cols,
                rows,
                ..
            } => {
                assert_eq!(session_id, "test-id");
                assert_eq!(cols, 80);
                assert_eq!(rows, 24);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn roundtrip_response_variants() {
        let variants: Vec<SupervisorResponse> = vec![
            SupervisorResponse::Spawned {
                session_id: "x".into(),
                pid: 42,
            },
            SupervisorResponse::Resized,
            SupervisorResponse::Killed,
            SupervisorResponse::Pong,
            SupervisorResponse::Error {
                message: "oops".into(),
            },
            SupervisorResponse::SessionList {
                sessions: vec![SupervisorSessionInfo {
                    session_id: "s".into(),
                    pid: 1,
                    alive: true,
                }],
            },
        ];
        for v in variants {
            let bytes = serde_json::to_vec(&v).unwrap();
            let _: SupervisorResponse = serde_json::from_slice(&bytes).unwrap();
        }
    }

    #[test]
    fn frame_encode_decode() {
        let data = b"hello world";
        let mut buf = Vec::new();
        write_frame(&mut buf, data).unwrap();
        assert_eq!(&buf[..4], &(11u32.to_le_bytes()));
        let mut cursor = std::io::Cursor::new(&buf);
        let decoded = read_frame(&mut cursor).unwrap();
        assert_eq!(decoded, data);
    }
}
