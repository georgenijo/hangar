use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, OwnedFd};

use anyhow::Result;

pub struct RawFdMaster {
    fd: OwnedFd,
}

impl RawFdMaster {
    pub fn new(fd: OwnedFd) -> Self {
        RawFdMaster { fd }
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), libc::TIOCSWINSZ, &ws) };
        if ret < 0 {
            anyhow::bail!("TIOCSWINSZ failed: {}", io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Read for RawFdMaster {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        nix::unistd::read(self.fd.as_raw_fd(), buf).map_err(io::Error::from)
    }
}

impl Write for RawFdMaster {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        nix::unistd::write(&self.fd, buf).map_err(io::Error::from)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
