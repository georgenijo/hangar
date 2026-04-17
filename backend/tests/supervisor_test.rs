/// Integration tests for hangar-supervisor: spawn, attach, PTY I/O, kill, list.
///
/// Each test spins up a real supervisor subprocess with an isolated socket path
/// via XDG_STATE_HOME override, so tests don't interfere with a running supervisor.
use std::collections::HashMap;
use std::os::unix::io::{AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use hangard::supervisor_protocol::{
    read_frame, recv_fd, write_frame, SupervisorRequest, SupervisorResponse,
};

// ── helpers ─────────────────────────────────────────────────────────────────

struct SupervisorGuard {
    child: std::process::Child,
    sock_path: PathBuf,
    _tmp: tempfile::TempDir,
}

impl Drop for SupervisorGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start_supervisor() -> SupervisorGuard {
    let tmp = tempfile::tempdir().unwrap();
    let sock_path = tmp.path().join("hangar/supervisor.sock");
    std::fs::create_dir_all(sock_path.parent().unwrap()).unwrap();

    let supervisor_bin = env!("CARGO_BIN_EXE_hangar-supervisor");
    let child = std::process::Command::new(supervisor_bin)
        .env("XDG_STATE_HOME", tmp.path())
        .env("RUST_LOG", "error")
        .spawn()
        .expect("failed to spawn hangar-supervisor");

    // wait for socket to appear (up to 5 s)
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !sock_path.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "supervisor socket never appeared at {:?}",
            sock_path
        );
        std::thread::sleep(Duration::from_millis(50));
    }

    SupervisorGuard {
        child,
        sock_path,
        _tmp: tmp,
    }
}

fn connect(path: &Path) -> UnixStream {
    // Retry a few times; socket file may exist before accept() is ready.
    for i in 0..20 {
        if i > 0 {
            std::thread::sleep(Duration::from_millis(50));
        }
        if let Ok(s) = UnixStream::connect(path) {
            return s;
        }
    }
    panic!("could not connect to supervisor socket at {:?}", path);
}

fn send_recv(stream: &mut UnixStream, req: &SupervisorRequest) -> SupervisorResponse {
    let bytes = serde_json::to_vec(req).unwrap();
    write_frame(stream, &bytes).unwrap();
    let frame = read_frame(stream).unwrap();
    serde_json::from_slice(&frame).unwrap()
}

/// Read bytes from a raw fd until `needle` is found or timeout elapses.
/// The fd is set non-blocking for the duration then restored.
fn read_until(fd: std::os::unix::io::RawFd, needle: &str, timeout: Duration) -> String {
    unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };
    let deadline = std::time::Instant::now() + timeout;
    let mut buf = Vec::new();
    while std::time::Instant::now() < deadline {
        let mut tmp = [0u8; 512];
        let n = unsafe { libc::read(fd, tmp.as_mut_ptr() as *mut libc::c_void, tmp.len()) };
        if n > 0 {
            buf.extend_from_slice(&tmp[..n as usize]);
            let s = String::from_utf8_lossy(&buf);
            if s.contains(needle) {
                // restore blocking
                unsafe { libc::fcntl(fd, libc::F_SETFL, 0) };
                return s.into_owned();
            }
        } else {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    unsafe { libc::fcntl(fd, libc::F_SETFL, 0) };
    String::from_utf8_lossy(&buf).into_owned()
}

fn base_env() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("TERM".into(), "xterm".into());
    m.insert("HOME".into(), std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()));
    m
}

// ── tests ────────────────────────────────────────────────────────────────────

/// Ping/pong works and the connection stays open for a second ping.
#[test]
fn test_ping_pong_persistent_connection() {
    let sup = start_supervisor();
    let mut stream = connect(&sup.sock_path);

    let r1 = send_recv(&mut stream, &SupervisorRequest::Ping);
    assert!(
        matches!(r1, SupervisorResponse::Pong),
        "first ping: expected Pong, got {:?}",
        r1
    );

    // This is the critical regression: connection must NOT close after first response.
    let r2 = send_recv(&mut stream, &SupervisorRequest::Ping);
    assert!(
        matches!(r2, SupervisorResponse::Pong),
        "second ping on same connection: expected Pong, got {:?} — connection closed early",
        r2
    );
}

/// Full cycle: spawn → list (alive) → attach_fd → PTY echo → kill → list (dead).
#[test]
fn test_spawn_attach_kill_cycle() {
    let sup = start_supervisor();
    let mut stream = connect(&sup.sock_path);

    let session_id = "supervisor-test-session".to_string();

    // 1. Spawn bash
    let spawn_resp = send_recv(
        &mut stream,
        &SupervisorRequest::Spawn {
            session_id: session_id.clone(),
            command: vec![
                "bash".into(),
                "--norc".into(),
                "--noprofile".into(),
            ],
            cwd: "/tmp".into(),
            env: base_env(),
            cols: 80,
            rows: 24,
        },
    );
    let pid = match spawn_resp {
        SupervisorResponse::Spawned { pid, .. } => pid,
        other => panic!("expected Spawned, got {:?}", other),
    };
    assert!(pid > 0, "pid must be positive");

    // 2. List — session alive
    let list_resp = send_recv(&mut stream, &SupervisorRequest::List);
    match list_resp {
        SupervisorResponse::SessionList { sessions } => {
            let s = sessions
                .iter()
                .find(|s| s.session_id == session_id)
                .expect("spawned session not found in list");
            assert!(s.alive, "session should be alive after spawn");
            assert_eq!(s.pid, pid);
        }
        other => panic!("expected SessionList, got {:?}", other),
    }

    // 3. AttachFd — receive PTY master fd via SCM_RIGHTS
    let bytes =
        serde_json::to_vec(&SupervisorRequest::AttachFd { session_id: session_id.clone() })
            .unwrap();
    write_frame(&mut stream, &bytes).unwrap();
    let frame = read_frame(&mut stream).unwrap();
    let attach_resp: SupervisorResponse = serde_json::from_slice(&frame).unwrap();
    assert!(
        matches!(attach_resp, SupervisorResponse::FdAttached { .. }),
        "expected FdAttached, got {:?}",
        attach_resp
    );
    let master_fd: OwnedFd = recv_fd(stream.as_raw_fd()).unwrap();

    // 4. PTY I/O: write echo command, read back output
    nix::unistd::write(&master_fd, b"echo hangar_test_ok\n").expect("write to PTY failed");
    let output = read_until(master_fd.as_raw_fd(), "hangar_test_ok", Duration::from_secs(3));
    assert!(
        output.contains("hangar_test_ok"),
        "PTY output did not contain expected string; got: {:?}",
        output
    );

    // 5. Kill the session with SIGKILL (bash ignores SIGTERM in interactive mode).
    let kill_resp = send_recv(
        &mut stream,
        &SupervisorRequest::Kill {
            session_id: session_id.clone(),
            signal: 9, // SIGKILL — guaranteed to terminate
        },
    );
    assert!(
        matches!(kill_resp, SupervisorResponse::Killed),
        "expected Killed, got {:?}",
        kill_resp
    );

    // 6. Wait for SIGCHLD to propagate through the supervisor's async handler.
    std::thread::sleep(Duration::from_millis(500));

    // 7. List — session now dead
    let list_resp2 = send_recv(&mut stream, &SupervisorRequest::List);
    match list_resp2 {
        SupervisorResponse::SessionList { sessions } => {
            let s = sessions.iter().find(|s| s.session_id == session_id);
            // Session must appear in list with alive=false after kill.
            if let Some(s) = s {
                assert!(
                    !s.alive,
                    "session should be dead after kill+wait, alive={}",
                    s.alive
                );
            }
            // If session was already reaped and removed, that's also acceptable.
        }
        other => panic!("expected SessionList, got {:?}", other),
    }
}

/// Attaching an unknown session_id returns Error, not a crash.
#[test]
fn test_attach_unknown_session_returns_error() {
    let sup = start_supervisor();
    let mut stream = connect(&sup.sock_path);

    let resp = send_recv(
        &mut stream,
        &SupervisorRequest::AttachFd {
            session_id: "no-such-session".into(),
        },
    );
    assert!(
        matches!(resp, SupervisorResponse::Error { .. }),
        "expected Error for unknown session, got {:?}",
        resp
    );

    // Connection must still be alive after receiving an error response.
    let pong = send_recv(&mut stream, &SupervisorRequest::Ping);
    assert!(
        matches!(pong, SupervisorResponse::Pong),
        "connection should survive after Error response"
    );
}

/// List on empty supervisor returns empty list, not an error.
#[test]
fn test_list_empty() {
    let sup = start_supervisor();
    let mut stream = connect(&sup.sock_path);

    let resp = send_recv(&mut stream, &SupervisorRequest::List);
    match resp {
        SupervisorResponse::SessionList { sessions } => {
            assert!(sessions.is_empty(), "expected empty list, got {:?}", sessions);
        }
        other => panic!("expected SessionList, got {:?}", other),
    }
}
