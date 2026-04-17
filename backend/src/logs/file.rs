use std::path::PathBuf;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::broadcast;
use tracing::warn;

use super::LogLine;

pub struct FileTailer {
    pub source_name: String,
    pub path: PathBuf,
    pub tail_lines: usize,
}

impl FileTailer {
    pub async fn initial_tail(&self) -> Vec<LogLine> {
        let content = match tokio::fs::read_to_string(&self.path).await {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let now = now_micros();
        let all: Vec<LogLine> = content
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| make_line(&self.source_name, l, now))
            .collect();
        let skip = all.len().saturating_sub(self.tail_lines);
        all.into_iter().skip(skip).collect()
    }

    pub async fn run(self, tx: broadcast::Sender<LogLine>) {
        let mut backoff_secs = 1u64;
        loop {
            match run_file_tailer(&self.source_name, &self.path, &tx).await {
                Ok(()) => break,
                Err(e) => {
                    warn!(
                        "file tailer '{}' error: {e}, retry in {backoff_secs}s",
                        self.source_name
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(30);
                }
            }
        }
    }
}

async fn run_file_tailer(
    source_name: &str,
    path: &PathBuf,
    tx: &broadcast::Sender<LogLine>,
) -> anyhow::Result<()> {
    let (notify_tx, mut notify_rx) =
        tokio::sync::mpsc::unbounded_channel::<notify::Result<notify::Event>>();

    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("no parent dir for {:?}", path))?
        .to_path_buf();

    let ntx = notify_tx.clone();
    let mut watcher = RecommendedWatcher::new(
        move |e| {
            let _ = ntx.send(e);
        },
        Config::default(),
    )?;
    watcher.watch(&parent, RecursiveMode::NonRecursive)?;

    let mut pos: u64 = match tokio::fs::metadata(path).await {
        Ok(m) => m.len(),
        Err(_) => 0,
    };

    loop {
        match notify_rx.recv().await {
            None => break,
            Some(Err(e)) => {
                warn!("notify error for '{}': {e}", source_name);
                return Err(e.into());
            }
            Some(Ok(event)) => {
                let relevant = event.paths.iter().any(|p| p == path);
                if !relevant {
                    continue;
                }
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        let new_len = match tokio::fs::metadata(path).await {
                            Ok(m) => m.len(),
                            Err(_) => continue,
                        };
                        if new_len < pos {
                            pos = 0; // truncated
                        }
                        if new_len <= pos {
                            continue;
                        }
                        let to_read = (new_len - pos) as usize;
                        let mut buf = vec![0u8; to_read];
                        match tokio::fs::File::open(path).await {
                            Ok(mut f) => {
                                if f.seek(std::io::SeekFrom::Start(pos)).await.is_ok() {
                                    match f.read_exact(&mut buf).await {
                                        Ok(_) => {
                                            pos = new_len;
                                            let now = now_micros();
                                            for line in String::from_utf8_lossy(&buf).lines() {
                                                let trimmed = line.trim();
                                                if !trimmed.is_empty() {
                                                    let _ = tx.send(make_line(
                                                        source_name,
                                                        trimmed,
                                                        now,
                                                    ));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::debug!("read error for '{source_name}': {e}");
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::debug!("open error for '{source_name}': {e}");
                            }
                        }
                    }
                    EventKind::Remove(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                        if tokio::fs::metadata(path).await.is_ok() {
                            pos = 0;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn make_line(source: &str, body: &str, ts_us: i64) -> LogLine {
    LogLine {
        source: source.to_string(),
        ts_us,
        level: detect_level(body),
        body: body.to_string(),
        unit: None,
    }
}

fn detect_level(line: &str) -> u8 {
    let u = line.to_uppercase();
    if u.contains("FATAL") || u.contains("ERROR") {
        3
    } else if u.contains("WARN") {
        4
    } else if u.contains("DEBUG") || u.contains("TRACE") {
        7
    } else {
        6
    }
}

fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as i64
}
