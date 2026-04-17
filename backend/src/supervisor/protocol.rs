use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const MAX_FRAME_SIZE: u32 = 1_048_576;
pub const FRAME_HEADER_LEN: usize = 4;

/// Newtype around Vec<u8> that serializes as base64.
#[derive(Debug, Clone)]
pub struct Base64Bytes(pub Vec<u8>);

impl Base64Bytes {
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl From<Vec<u8>> for Base64Bytes {
    fn from(v: Vec<u8>) -> Self {
        Base64Bytes(v)
    }
}

impl From<&[u8]> for Base64Bytes {
    fn from(v: &[u8]) -> Self {
        Base64Bytes(v.to_vec())
    }
}

impl Serialize for Base64Bytes {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&self.0);
        s.serialize_str(&encoded)
    }
}

impl<'de> Deserialize<'de> for Base64Bytes {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use base64::Engine;
        let s = String::deserialize(d)?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)?;
        Ok(Base64Bytes(bytes))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum SupervisorCmd {
    Spawn {
        id: String,
        slug: String,
        command: Vec<String>,
        env: BTreeMap<String, String>,
        cwd: String,
        cols: u16,
        rows: u16,
    },
    AttachFd {
        id: String,
        nonce: u64,
    },
    Write {
        id: String,
        data: Base64Bytes,
    },
    Resize {
        id: String,
        cols: u16,
        rows: u16,
    },
    Kill {
        id: String,
        signal: i32,
    },
    List,
    RegisterFd {
        id: String,
        pid: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum SupervisorResp {
    Spawned { id: String, pid: u32 },
    FdReady { nonce: u64 },
    Written,
    Resized,
    Killed,
    Sessions(Vec<SessionInfo>),
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub slug: String,
    pub pid: u32,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandshakeRole {
    Primary,
    FdChannel,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Handshake {
    pub client_id: uuid::Uuid,
    pub role: HandshakeRole,
}

pub async fn read_frame<R: tokio::io::AsyncRead + Unpin>(r: &mut R) -> anyhow::Result<Vec<u8>> {
    let mut header = [0u8; FRAME_HEADER_LEN];
    r.read_exact(&mut header).await?;
    let len = u32::from_be_bytes(header);
    if len > MAX_FRAME_SIZE {
        anyhow::bail!("frame too large: {} > {}", len, MAX_FRAME_SIZE);
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf).await?;
    Ok(buf)
}

pub async fn write_frame<W: tokio::io::AsyncWrite + Unpin>(
    w: &mut W,
    data: &[u8],
) -> anyhow::Result<()> {
    let len = data.len() as u32;
    w.write_all(&len.to_be_bytes()).await?;
    w.write_all(data).await?;
    Ok(())
}
