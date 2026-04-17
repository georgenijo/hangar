//! Integration tests for supervisor fd-passing and session survival.
//! Run with: cargo test -- --ignored

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

async fn wait_for_socket(path: &PathBuf, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

fn build_supervisor_cmd(socket_path: &PathBuf) -> std::process::Command {
    let mut cmd = std::process::Command::new(
        std::env::var("CARGO_BIN_EXE_hangar-supervisor")
            .unwrap_or_else(|_| "target/debug/hangar-supervisor".to_string()),
    );
    cmd.arg("--socket-path").arg(socket_path);
    cmd
}

#[tokio::test]
#[ignore]
async fn test_session_survives_backend_restart() {
    let tmp = tempfile::TempDir::new().unwrap();
    let socket_path = tmp.path().join("supervisor.sock");

    let mut supervisor = build_supervisor_cmd(&socket_path)
        .env("HOME", tmp.path())
        .spawn()
        .expect("failed to spawn supervisor");

    let appeared = wait_for_socket(&socket_path, Duration::from_secs(5)).await;
    assert!(appeared, "supervisor socket did not appear");

    // First client: spawn a session
    let handle = hangar_backend::supervisor::SupervisorHandle::connect(&socket_path)
        .await
        .expect("connect failed");

    let mut env = BTreeMap::new();
    env.insert(
        "HOME".to_string(),
        tmp.path().to_str().unwrap().to_string(),
    );

    let resp = handle
        .spawn(hangar_backend::supervisor::protocol::SupervisorCmd::Spawn {
            id: "test-1".to_string(),
            slug: "test".to_string(),
            command: vec![
                "bash".to_string(),
                "-c".to_string(),
                "while true; do echo heartbeat; sleep 1; done".to_string(),
            ],
            env,
            cwd: tmp.path().to_str().unwrap().to_string(),
            cols: 80,
            rows: 24,
        })
        .await
        .expect("spawn failed");

    let pid = match resp {
        hangar_backend::supervisor::protocol::SupervisorResp::Spawned { pid, .. } => pid,
        r => panic!("expected Spawned, got {:?}", r),
    };

    // Attach fd and read output
    let master_fd = handle.attach_fd("test-1").await.expect("attach_fd failed");

    // Read some output
    use std::os::unix::io::AsRawFd;
    let raw = master_fd.as_raw_fd();
    let mut output = [0u8; 256];
    // Non-blocking read attempt
    nix::fcntl::fcntl(raw, nix::fcntl::FcntlArg::F_SETFL(nix::fcntl::OFlag::O_NONBLOCK))
        .unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let n = unsafe { libc::read(raw, output.as_mut_ptr() as *mut libc::c_void, output.len()) };
    assert!(n > 0, "expected output from child");
    let text = std::str::from_utf8(&output[..n as usize]).unwrap_or("");
    assert!(
        text.contains("heartbeat"),
        "expected 'heartbeat' in output, got: {:?}",
        text
    );

    // Suppress unused variable warning
    let _ = pid;

    // Drop handle (simulates backend disconnect)
    drop(handle);
    drop(master_fd);
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Reconnect
    let handle2 = hangar_backend::supervisor::SupervisorHandle::connect(&socket_path)
        .await
        .expect("reconnect failed");

    let sessions = handle2.list().await.expect("list failed");
    assert!(
        sessions.iter().any(|s| s.id == "test-1" && s.running),
        "session test-1 should still be running after reconnect: {:?}",
        sessions
    );

    // Kill session and cleanup
    handle2.kill_session("test-1", libc::SIGTERM).await.ok();
    supervisor.kill().unwrap();
}

#[tokio::test]
#[ignore]
async fn test_supervisor_crash_backend_holds_fd() {
    let tmp = tempfile::TempDir::new().unwrap();
    let socket_path = tmp.path().join("supervisor.sock");

    let mut supervisor = build_supervisor_cmd(&socket_path)
        .env("HOME", tmp.path())
        .spawn()
        .expect("failed to spawn supervisor");

    wait_for_socket(&socket_path, Duration::from_secs(5)).await;

    let handle = hangar_backend::supervisor::SupervisorHandle::connect(&socket_path)
        .await
        .unwrap();

    let mut env = BTreeMap::new();
    env.insert(
        "HOME".to_string(),
        tmp.path().to_str().unwrap().to_string(),
    );

    handle
        .spawn(hangar_backend::supervisor::protocol::SupervisorCmd::Spawn {
            id: "crash-test".to_string(),
            slug: "crash-test".to_string(),
            command: vec![
                "bash".to_string(),
                "-c".to_string(),
                "while true; do echo alive; sleep 1; done".to_string(),
            ],
            env,
            cwd: tmp.path().to_str().unwrap().to_string(),
            cols: 80,
            rows: 24,
        })
        .await
        .unwrap();

    let master_fd = handle.attach_fd("crash-test").await.unwrap();

    // Kill supervisor
    supervisor.kill().unwrap();
    supervisor.wait().unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify fd still works
    use std::os::unix::io::AsRawFd;
    let raw = master_fd.as_raw_fd();
    nix::fcntl::fcntl(raw, nix::fcntl::FcntlArg::F_SETFL(nix::fcntl::OFlag::O_NONBLOCK))
        .unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let mut buf = [0u8; 256];
    let n = unsafe { libc::read(raw, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    assert!(n > 0, "fd should still be readable after supervisor crash");

    // Cleanup
    let _ = std::fs::remove_file(&socket_path);
}
