use anyhow::{bail, Result};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;
use std::path::Path;

pub const MAGIC: [u8; 4] = *b"HNGR";
pub const VERSION: u8 = 0x01;
pub const HEADER_SIZE: u64 = 16;
pub const DEFAULT_CAPACITY: u64 = 104_857_600;

pub struct RingBuf {
    file: File,
    capacity: u64,
    head: u64,
}

impl RingBuf {
    pub fn create(path: &Path, capacity: u64) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        file.set_len(HEADER_SIZE + capacity)?;

        let mut header = [0u8; 16];
        header[0..4].copy_from_slice(&MAGIC);
        header[4] = VERSION;
        // bytes 5..8 are pad = 0x00
        // bytes 8..16 are head = 0u64 LE (already zero)
        file.write_at(&header, 0)?;
        file.sync_data()?;

        Ok(RingBuf {
            file,
            capacity,
            head: 0,
        })
    }

    pub fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;

        let mut header = [0u8; 16];
        file.read_at(&mut header, 0)?;

        if &header[0..4] != &MAGIC {
            bail!("invalid magic bytes");
        }
        if header[4] != VERSION {
            bail!("unsupported version: {}", header[4]);
        }

        let head = u64::from_le_bytes(header[8..16].try_into()?);
        let file_len = file.metadata()?.len();
        if file_len < HEADER_SIZE {
            bail!("file too small");
        }
        let capacity = file_len - HEADER_SIZE;

        Ok(RingBuf {
            file,
            capacity,
            head,
        })
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(u64, u32)> {
        let len = data.len();
        if len as u64 > self.capacity {
            bail!(
                "write ({} bytes) exceeds ring capacity ({} bytes)",
                len,
                self.capacity
            );
        }

        let offset = self.head;
        let file_pos = self.head % self.capacity;

        if file_pos + len as u64 <= self.capacity {
            self.file.write_at(data, HEADER_SIZE + file_pos)?;
        } else {
            let first_len = (self.capacity - file_pos) as usize;
            self.file
                .write_at(&data[..first_len], HEADER_SIZE + file_pos)?;
            self.file.write_at(&data[first_len..], HEADER_SIZE)?;
        }

        self.head += len as u64;

        let head_bytes = self.head.to_le_bytes();
        self.file.write_at(&head_bytes, 8)?;

        Ok((offset, len as u32))
    }

    pub fn sync(&self) -> Result<()> {
        self.file.sync_data()?;
        Ok(())
    }

    pub fn read_at(&self, offset: u64, len: u32) -> Result<Vec<u8>> {
        if len == 0 {
            return Ok(Vec::new());
        }

        if self.head > self.capacity && self.head - offset > self.capacity {
            bail!(
                "data at offset {} has been overwritten (head={})",
                offset,
                self.head
            );
        }

        let file_pos = offset % self.capacity;
        let mut buf = vec![0u8; len as usize];

        if file_pos + len as u64 <= self.capacity {
            self.file.read_at(&mut buf, HEADER_SIZE + file_pos)?;
        } else {
            let first_len = (self.capacity - file_pos) as usize;
            self.file
                .read_at(&mut buf[..first_len], HEADER_SIZE + file_pos)?;
            self.file.read_at(&mut buf[first_len..], HEADER_SIZE)?;
        }

        Ok(buf)
    }

    pub fn head(&self) -> u64 {
        self.head
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}
