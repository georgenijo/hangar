//! Spike binary that validates the SCM_RIGHTS fd-passing flow end-to-end.
//!
//! Spawns bash via openpty, passes the master fd over a Unix socket to a
//! second thread (simulating the backend role), writes a command, and checks
//! output contains the expected string.

use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::net::UnixListener;

use anyhow::Result;
use nix::sys::socket::{recvmsg, sendmsg, ControlMessage, ControlMessageOwned, MsgFlags, UnixAddr};

fn main() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let sock_path = dir.path().join("test.sock");

    let listener = UnixListener::bind(&sock_path)?;
    let sock_path_clone = sock_path.clone();

    // --- Supervisor role (thread 1) ---
    let supervisor = std::thread::spawn(move || -> Result<()> {
        // Open PTY
        let ws = nix::pty::Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let pty = nix::pty::openpty(Some(&ws), None)?;
        let slave_raw = pty.slave.as_raw_fd();

        // Spawn bash
        use std::os::unix::process::CommandExt;
        let mut cmd = std::process::Command::new("/bin/bash");
        cmd.env_clear().env("TERM", "xterm");
        unsafe {
            cmd.pre_exec(move || {
                libc::setsid();
                libc::ioctl(slave_raw, libc::TIOCSCTTY.into(), 0i32);
                libc::dup2(slave_raw, 0);
                libc::dup2(slave_raw, 1);
                libc::dup2(slave_raw, 2);
                for fd in 3..256i32 {
                    libc::close(fd);
                }
                Ok(())
            });
        }
        let child = cmd.spawn()?;
        drop(pty.slave);
        let _pid = child.id();
        drop(child);

        // Wait for backend connection
        let (mut conn, _) = listener.accept()?;

        // Wait for "ready" byte
        let mut buf = [0u8; 1];
        conn.read_exact(&mut buf)?;

        // Send master fd via SCM_RIGHTS
        let fds = [pty.master.as_raw_fd()];
        let cmsg = [ControlMessage::ScmRights(&fds)];
        let iov = [std::io::IoSlice::new(b"\x00")];
        sendmsg::<UnixAddr>(conn.as_raw_fd(), &iov, &cmsg, MsgFlags::empty(), None)?;

        // Keep alive briefly
        std::thread::sleep(std::time::Duration::from_secs(2));
        Ok(())
    });

    // --- Backend role (thread 2) ---
    let backend = std::thread::spawn(move || -> Result<()> {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut conn = std::os::unix::net::UnixStream::connect(&sock_path_clone)?;

        // Signal ready
        conn.write_all(b"\x00")?;

        // Receive fd
        let mut data_buf = [0u8; 1];
        let mut iov = [std::io::IoSliceMut::new(&mut data_buf)];
        let mut cmsg_space = nix::cmsg_space!([std::os::unix::io::RawFd; 1]);
        let msg = recvmsg::<UnixAddr>(
            conn.as_raw_fd(),
            &mut iov,
            Some(&mut cmsg_space),
            MsgFlags::empty(),
        )?;

        let mut received_fd: Option<OwnedFd> = None;
        for cmsg in msg.cmsgs().expect("cmsgs") {
            if let ControlMessageOwned::ScmRights(fds) = cmsg {
                if let Some(&raw) = fds.first() {
                    received_fd = Some(unsafe { OwnedFd::from_raw_fd(raw) });
                }
            }
        }
        let fd = received_fd.expect("no fd received");

        // Write command to the PTY master
        let mut f = unsafe { std::fs::File::from_raw_fd(fd.as_raw_fd()) };
        std::thread::sleep(std::time::Duration::from_millis(200));
        f.write_all(b"echo spike-test-ok\n")?;

        // Read output with timeout
        let mut out = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut tmp = [0u8; 256];
        loop {
            // Non-blocking read attempt
            match nix::unistd::read(fd.as_raw_fd(), &mut tmp) {
                Ok(0) => break,
                Ok(n) => {
                    out.extend_from_slice(&tmp[..n]);
                    if String::from_utf8_lossy(&out).contains("spike-test-ok") {
                        break;
                    }
                }
                Err(_) => {}
            }
            if std::time::Instant::now() > deadline {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Prevent double-close (File::drop vs OwnedFd::drop)
        std::mem::forget(f);

        let output = String::from_utf8_lossy(&out);
        assert!(
            output.contains("spike-test-ok"),
            "expected 'spike-test-ok' in output, got: {:?}",
            output
        );
        println!("fd-passing spike: PASS");
        Ok(())
    });

    supervisor.join().unwrap()?;
    backend.join().unwrap()?;
    Ok(())
}
