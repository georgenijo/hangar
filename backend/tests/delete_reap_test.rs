// Regression test for #51: DELETE /sessions/:id must reap the PTY child so
// no zombie (state `Z`) remains parented to this (the test hangard) process.
//
// Uses the shell driver so no external binary (claude) is required. The test
// spawns a real PTY via the in-process axum server, sends a long-running
// prompt (`yes | head -n ...`) so the child is actively writing at DELETE
// time, then asserts the PID disappears from /proc within a short grace
// window and is not left in `Z` state.
//
// Marked `#[ignore]` like the other PTY tests because it allocates a real
// terminal — run with: `cargo test -p hangard delete_reap -- --ignored`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use hangard::{api, db::Db, events::EventBus, AppState};

async fn spawn_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let db = Db::new_in_memory().await.unwrap();
    let event_bus = Arc::new(EventBus::new());
    let tmp = tempfile::tempdir().unwrap();
    let ring_dir = tmp.path().to_path_buf();
    std::mem::forget(tmp);

    let logs_config = hangard::config::LogsConfig::default();
    let mut logs_hub = hangard::logs::LogsHub::new(&logs_config, &ring_dir);
    logs_hub.start();

    let state = AppState {
        db,
        event_bus,
        ring_dir,
        hook_channels: Arc::new(Mutex::new(HashMap::new())),
        sessions: Arc::new(RwLock::new(HashMap::new())),
        supervisor: None,
        start_time: Instant::now(),
        sandbox_manager: None,
        logs: Arc::new(logs_hub),
    };

    let router = api::router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://127.0.0.1:{}", addr.port());

    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (base_url, handle)
}

/// Read /proc/self/task/*/children (or /proc/<ppid>/task/*/children on Linux
/// with CONFIG_PROC_CHILDREN) to find any child PIDs of the current process.
/// Falls back to walking /proc looking for matching PPIDs.
fn child_pids_of_self() -> Vec<i32> {
    let my_pid = std::process::id() as i32;
    let mut out = Vec::new();
    let entries = match std::fs::read_dir("/proc") {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Ok(pid) = name.parse::<i32>() else {
            continue;
        };
        let stat_path = format!("/proc/{pid}/stat");
        let stat = match std::fs::read_to_string(&stat_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        // /proc/<pid>/stat format: "<pid> (<comm>) <state> <ppid> ..."
        // comm can contain spaces/parens; parse from the last ')'.
        let Some(close) = stat.rfind(')') else {
            continue;
        };
        let tail = &stat[close + 1..];
        let mut toks = tail.split_whitespace();
        let _state = toks.next();
        let Some(ppid_str) = toks.next() else {
            continue;
        };
        let Ok(ppid) = ppid_str.parse::<i32>() else {
            continue;
        };
        if ppid == my_pid {
            out.push(pid);
        }
    }
    out
}

fn proc_state(pid: i32) -> Option<char> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close = stat.rfind(')')?;
    let tail = &stat[close + 1..];
    tail.split_whitespace().next()?.chars().next()
}

#[tokio::test]
#[ignore = "requires PTY allocation; run manually with: cargo test -- --ignored"]
async fn delete_mid_stream_leaves_no_zombies() {
    let (base, _server) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Baseline — record any pre-existing children so we can ignore them.
    let baseline: std::collections::HashSet<i32> = child_pids_of_self().into_iter().collect();

    // Create a shell session.
    let resp = client
        .post(format!("{base}/api/v1/sessions"))
        .json(&serde_json::json!({
            "slug": "reap-test",
            "kind": {"type": "shell"},
            "cols": 80,
            "rows": 24
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let session: serde_json::Value = resp.json().await.unwrap();
    let id = session["id"].as_str().unwrap().to_string();

    // Let the PTY come up so `ps`/`/proc` will list the child.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let new_children: Vec<i32> = child_pids_of_self()
        .into_iter()
        .filter(|p| !baseline.contains(p))
        .collect();
    assert!(
        !new_children.is_empty(),
        "expected at least one PTY child after spawn"
    );

    // Kick off a long-running stream so DELETE fires mid-output.
    let resp = client
        .post(format!("{base}/api/v1/sessions/{id}/prompt"))
        .json(&serde_json::json!({"text": "yes hangar | head -n 100000"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Small beat, then DELETE mid-stream.
    tokio::time::sleep(Duration::from_millis(80)).await;

    let resp = client
        .delete(format!("{base}/api/v1/sessions/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Grace window for the reaper task. reap_child uses ~500ms; give it
    // comfortably more so a slow shell-exit doesn't flake the test.
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let still: Vec<(i32, Option<char>)> = new_children
            .iter()
            .filter_map(|p| {
                let st = proc_state(*p);
                // Gone from /proc entirely → fully reaped.
                if st.is_none() {
                    None
                } else {
                    Some((*p, st))
                }
            })
            .collect();

        if still.is_empty() {
            return;
        }

        // Any surviving entry must not be a zombie.
        let zombies: Vec<_> = still.iter().filter(|(_, s)| *s == Some('Z')).collect();
        if zombies.is_empty() && Instant::now() > deadline {
            panic!(
                "children still alive after grace (not zombies, but not reaped either): {:?}",
                still
            );
        }
        if !zombies.is_empty() && Instant::now() > deadline {
            panic!("zombie processes parented to test hangard after DELETE: {zombies:?}");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
