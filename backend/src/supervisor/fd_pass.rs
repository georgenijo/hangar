#[cfg(target_os = "linux")]
mod linux {
    use nix::fcntl::{fcntl, FcntlArg, FdFlag};
    use nix::sys::socket::{recvmsg, sendmsg, ControlMessage, ControlMessageOwned, MsgFlags};
    use std::io::{IoSlice, IoSliceMut};
    use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};

    pub fn send_fd(socket: RawFd, fd: RawFd, payload: &[u8]) -> anyhow::Result<()> {
        // payload MUST be non-empty for SCM_RIGHTS
        let payload = if payload.is_empty() {
            b"\x00" as &[u8]
        } else {
            payload
        };
        let iov = [IoSlice::new(payload)];
        let fds = [fd];
        let cmsg = [ControlMessage::ScmRights(&fds)];
        sendmsg::<()>(socket, &iov, &cmsg, MsgFlags::empty(), None)?;
        Ok(())
    }

    pub fn recv_fd(
        socket: RawFd,
        buf: &mut [u8],
    ) -> anyhow::Result<(usize, Option<OwnedFd>)> {
        let mut iov = [IoSliceMut::new(buf)];
        let mut cmsg_buf = nix::cmsg_space!(RawFd);
        let msg = recvmsg::<()>(socket, &mut iov, Some(&mut cmsg_buf), MsgFlags::empty())?;
        let bytes = msg.bytes;
        let mut received_fd: Option<OwnedFd> = None;
        for cmsg in msg.cmsgs()? {
            if let ControlMessageOwned::ScmRights(fds) = cmsg {
                if let Some(&fd) = fds.first() {
                    let owned = unsafe { OwnedFd::from_raw_fd(fd) };
                    set_cloexec(owned.as_raw_fd())?;
                    received_fd = Some(owned);
                    break;
                }
            }
        }
        Ok((bytes, received_fd))
    }

    pub fn set_cloexec(fd: RawFd) -> anyhow::Result<()> {
        let flags = fcntl(fd, FcntlArg::F_GETFD)?;
        let new_flags = FdFlag::from_bits_truncate(flags) | FdFlag::FD_CLOEXEC;
        fcntl(fd, FcntlArg::F_SETFD(new_flags))?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub use linux::{recv_fd, send_fd, set_cloexec};

#[cfg(not(target_os = "linux"))]
pub fn send_fd(
    _socket: std::os::unix::io::RawFd,
    _fd: std::os::unix::io::RawFd,
    _payload: &[u8],
) -> anyhow::Result<()> {
    anyhow::bail!("fd passing not supported on this platform")
}

#[cfg(not(target_os = "linux"))]
pub fn recv_fd(
    _socket: std::os::unix::io::RawFd,
    _buf: &mut [u8],
) -> anyhow::Result<(usize, Option<std::os::unix::io::OwnedFd>)> {
    anyhow::bail!("fd passing not supported on this platform")
}

#[cfg(not(target_os = "linux"))]
pub fn set_cloexec(_fd: std::os::unix::io::RawFd) -> anyhow::Result<()> {
    anyhow::bail!("not supported on this platform")
}
