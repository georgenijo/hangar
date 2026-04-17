//! Dev tool to validate SCM_RIGHTS fd passing works end-to-end.
//! Build with: cargo build --features dev-tools --bin fd_pass_demo

use nix::sys::socket::{socketpair, AddressFamily, SockFlag, SockType};
use std::os::unix::io::AsRawFd;

fn main() -> anyhow::Result<()> {
    println!("fd_pass_demo: testing SCM_RIGHTS fd passing");

    // Create a socket pair simulating two "processes"
    let (sock_a, sock_b) = socketpair(
        AddressFamily::Unix,
        SockType::Stream,
        None,
        SockFlag::empty(),
    )?;

    // Create a temp file to pass
    let tmp = tempfile()?;
    let tmp_raw = tmp.as_raw_fd();

    println!("parent: temp file fd = {}", tmp_raw);

    // Send fd from sock_a to sock_b
    hangar_backend::supervisor::fd_pass::send_fd(sock_a.as_raw_fd(), tmp_raw, b"hello")?;
    println!("parent: sent fd {} via SCM_RIGHTS", tmp_raw);

    // Receive on sock_b
    let mut buf = [0u8; 64];
    let (n, received_fd) =
        hangar_backend::supervisor::fd_pass::recv_fd(sock_b.as_raw_fd(), &mut buf)?;

    let payload = std::str::from_utf8(&buf[..n]).unwrap_or("(non-utf8)");
    println!("child: received {} bytes payload: {:?}", n, payload);

    match received_fd {
        Some(fd) => {
            println!("child: received fd {}", fd.as_raw_fd());
            // Verify it's a valid fd by getting its flags
            let flags =
                nix::fcntl::fcntl(fd.as_raw_fd(), nix::fcntl::FcntlArg::F_GETFD)?;
            println!(
                "child: fd flags = {} (FD_CLOEXEC should be set)",
                flags
            );
            assert!(
                flags & nix::fcntl::FdFlag::FD_CLOEXEC.bits() != 0,
                "CLOEXEC not set!"
            );
            println!("PASS: fd received and CLOEXEC is set");
        }
        None => {
            println!("FAIL: no fd received");
            std::process::exit(1);
        }
    }

    println!("fd_pass_demo: all checks passed");
    Ok(())
}

fn tempfile() -> anyhow::Result<std::fs::File> {
    use std::io::Write;
    let path = "/tmp/fd_pass_demo_test";
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "test content")?;
    // Reopen for reading
    Ok(std::fs::File::open(path)?)
}
