//! Integration test for supervisor survive-restart (#37).
//!
//! Spins up a real `hangar-supervisor` and `hangard` on an ephemeral
//! socket / state dir / TCP port via the `HANGAR_SUPERVISOR_SOCK` +
//! `HANGAR_STATE_DIR` + `HANGAR_PORT` env overrides, creates a shell
//! session, writes bytes, kills hangard, starts a fresh hangard against
//! the same supervisor, and asserts the session survived and the PTY
//! still accepts I/O.
//!
//! Run: `cargo test --target-dir /tmp/hangar-ephemeral --test supervisor_restart -- --nocapture`

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;
use tempfile::TempDir;

// ── env / harness ──────────────────────────────────────────────────────────

struct Harness {
    _tmp: TempDir,
    state_dir: PathBuf,
    sock_path: PathBuf,
    config_dir: PathBuf,
    port: u16,
    base_url: String,
    stamp_path: PathBuf,
}

impl Harness {
    fn new() -> Self {
        let tmp = tempfile::tempdir().expect("mkdtemp");
        let root = tmp.path().to_path_buf();
        let state_dir = root.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let sock_path = state_dir.join("supervisor.sock");
        // Empty XDG_CONFIG_HOME so hangard falls back to default config (no sandbox, no push).
        let config_dir = root.join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let port = pick_free_port();
        let base_url = format!("http://127.0.0.1:{port}");
        let stamp_path = root.join("stamp.txt");
        Harness {
            _tmp: tmp,
            state_dir,
            sock_path,
            config_dir,
            port,
            base_url,
            stamp_path,
        }
    }
}

fn pick_free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}

// ── child guards ───────────────────────────────────────────────────────────

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_supervisor(h: &Harness) -> ChildGuard {
    let bin = env!("CARGO_BIN_EXE_hangar-supervisor");
    let child = Command::new(bin)
        .env("HANGAR_STATE_DIR", &h.state_dir)
        .env("HANGAR_SUPERVISOR_SOCK", &h.sock_path)
        .env("XDG_CONFIG_HOME", &h.config_dir)
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn hangar-supervisor");

    let deadline = Instant::now() + Duration::from_secs(5);
    while !h.sock_path.exists() {
        assert!(
            Instant::now() < deadline,
            "supervisor socket never appeared at {:?}",
            h.sock_path
        );
        std::thread::sleep(Duration::from_millis(30));
    }
    ChildGuard(child)
}

fn start_hangard(h: &Harness) -> ChildGuard {
    let bin = env!("CARGO_BIN_EXE_hangard");
    let child = Command::new(bin)
        .env("HANGAR_STATE_DIR", &h.state_dir)
        .env("HANGAR_SUPERVISOR_SOCK", &h.sock_path)
        .env("HANGAR_PORT", h.port.to_string())
        .env("XDG_CONFIG_HOME", &h.config_dir)
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn hangard");
    ChildGuard(child)
}

// ── HTTP helpers ───────────────────────────────────────────────────────────

async fn wait_supervisor_connected(client: &reqwest::Client, base: &str) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(r) = client
            .get(format!("{base}/api/v1/health"))
            .timeout(Duration::from_millis(500))
            .send()
            .await
        {
            if let Ok(v) = r.json::<Value>().await {
                if v.get("supervisor_connected") == Some(&Value::Bool(true)) {
                    return;
                }
            }
        }
        assert!(
            Instant::now() < deadline,
            "hangard never reported supervisor_connected=true"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_hangard_down(client: &reqwest::Client, base: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let down = client
            .get(format!("{base}/api/v1/health"))
            .timeout(Duration::from_millis(200))
            .send()
            .await
            .is_err();
        if down {
            return;
        }
        assert!(Instant::now() < deadline, "old hangard never went down");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn get_session_state(client: &reqwest::Client, base: &str, sid: &str) -> String {
    let r = client
        .get(format!("{base}/api/v1/sessions/{sid}"))
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success(), "GET session: {}", r.status());
    let v: Value = r.json().await.unwrap();
    v["state"].as_str().unwrap().to_string()
}

fn wait_for_contents(path: &Path, needle: &str, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(s) = std::fs::read_to_string(path) {
            if s.contains(needle) {
                return;
            }
        }
        if Instant::now() > deadline {
            let got = std::fs::read_to_string(path).unwrap_or_default();
            panic!("{path:?} never contained {needle:?}; got: {got:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn sigterm(child: &ChildGuard) {
    let pid = child.0.id() as i32;
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid),
        nix::sys::signal::Signal::SIGTERM,
    );
}

// ── the test ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn shell_session_survives_hangard_restart() {
    let h = Harness::new();
    let _sup = start_supervisor(&h);
    let mut hangard1 = start_hangard(&h);

    let client = reqwest::Client::new();
    wait_supervisor_connected(&client, &h.base_url).await;

    // 1. Create a shell session.
    let resp = client
        .post(format!("{}/api/v1/sessions", h.base_url))
        .json(&serde_json::json!({
            "slug": "sup-it",
            "kind": {"type": "shell"},
        }))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "create session: {}",
        resp.status()
    );
    let session: Value = resp.json().await.unwrap();
    let sid = session["id"].as_str().unwrap().to_string();

    // 2. Write a command that leaves a stamp on disk. /prompt is a raw
    //    PTY write for the shell driver.
    let stamp = h.stamp_path.display().to_string();
    let pre_cmd = format!("echo SUP_BEFORE > {stamp}\n");
    let r = client
        .post(format!("{}/api/v1/sessions/{}/prompt", h.base_url, sid))
        .json(&serde_json::json!({ "text": pre_cmd }))
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success(), "pre-write: {}", r.status());
    wait_for_contents(&h.stamp_path, "SUP_BEFORE", Duration::from_secs(10));

    // 3. Record pre-restart state.
    let state_before = get_session_state(&client, &h.base_url, &sid).await;
    assert_ne!(
        state_before, "exited",
        "session should not be exited pre-restart"
    );
    let pid_before = hangard1.0.id();

    // 4. SIGTERM hangard and wait for it to exit. Drop the guard so the
    //    port is free for the replacement.
    sigterm(&hangard1);
    let _ = hangard1.0.wait();
    drop(hangard1);
    wait_hangard_down(&client, &h.base_url).await;

    // 5. Start a fresh hangard pointed at the same supervisor, DB, ring.
    let _hangard2 = start_hangard(&h);
    wait_supervisor_connected(&client, &h.base_url).await;
    // health reports uptime — just confirm the bin is a new process (not
    //  strictly required, but nice to have an explicit check):
    let pid_after = _hangard2.0.id();
    assert_ne!(pid_before, pid_after, "hangard pid did not change");

    // 6. Session must still be non-exited.
    let state_after = get_session_state(&client, &h.base_url, &sid).await;
    assert_ne!(
        state_after, "exited",
        "session flipped to exited across restart (before={state_before}, after={state_after})"
    );

    // 7. Write again — if the supervisor truly re-passed the same PTY fd
    //    to the new hangard, the append lands on the same stamp file.
    let post_cmd = format!("echo SUP_AFTER >> {stamp}\n");
    let r = client
        .post(format!("{}/api/v1/sessions/{}/prompt", h.base_url, sid))
        .json(&serde_json::json!({ "text": post_cmd }))
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success(), "post-write: {}", r.status());
    wait_for_contents(&h.stamp_path, "SUP_AFTER", Duration::from_secs(10));

    let contents = std::fs::read_to_string(&h.stamp_path).unwrap();
    assert!(
        contents.contains("SUP_BEFORE") && contents.contains("SUP_AFTER"),
        "stamp file missing expected lines; got: {contents:?}"
    );
}
