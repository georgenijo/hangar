use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::broadcast;
use tracing::warn;

use super::LogLine;

pub struct JournalctlTailer {
    pub source_name: String,
    pub unit: Option<String>,
    pub tail_lines: usize,
}

impl JournalctlTailer {
    pub async fn initial_tail(&self) -> Vec<LogLine> {
        let mut cmd = Command::new("journalctl");
        cmd.args([
            "-n",
            &self.tail_lines.to_string(),
            "--output=json",
            "--no-pager",
        ]);
        if let Some(ref unit) = self.unit {
            cmd.arg(format!("--unit={unit}"));
        }
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                warn!("journalctl initial_tail spawn failed: {e}");
                return Vec::new();
            }
        };

        let output = match child.wait_with_output().await {
            Ok(o) => o,
            Err(e) => {
                warn!("journalctl initial_tail wait failed: {e}");
                return Vec::new();
            }
        };

        let mut lines: Vec<LogLine> = output
            .stdout
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .filter_map(|l| std::str::from_utf8(l).ok())
            .filter_map(|l| parse_journal_line(&self.source_name, l))
            .collect();

        lines.sort_by_key(|l| l.ts_us);
        lines
    }

    pub async fn run(self, tx: broadcast::Sender<LogLine>) {
        let mut backoff_secs = 1u64;
        loop {
            match run_live(&self.source_name, self.unit.as_deref(), &tx).await {
                Ok(()) => break,
                Err(e) => {
                    warn!(
                        "journalctl tailer '{}' error: {e}, retry in {backoff_secs}s",
                        self.source_name
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(30);
                }
            }
        }
    }
}

async fn run_live(
    source_name: &str,
    unit: Option<&str>,
    tx: &broadcast::Sender<LogLine>,
) -> anyhow::Result<()> {
    let mut cmd = Command::new("journalctl");
    cmd.args(["-f", "--output=json", "--since=now"]);
    if let Some(u) = unit {
        cmd.arg(format!("--unit={u}"));
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    let mut child = cmd.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("no stdout"))?;
    let mut lines = tokio::io::BufReader::new(stdout).lines();

    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if let Some(log_line) = parse_journal_line(source_name, &line) {
                    let _ = tx.send(log_line);
                }
            }
            Ok(None) => break,
            Err(e) => {
                tracing::debug!("journalctl read error for '{source_name}': {e}");
                break;
            }
        }
    }

    let _ = child.kill().await;
    let _ = child.wait().await;
    Err(anyhow::anyhow!("journalctl process exited"))
}

fn parse_journal_line(source_name: &str, line: &str) -> Option<LogLine> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;

    let body = match v.get("MESSAGE") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(bytes)) => String::from_utf8(
            bytes
                .iter()
                .filter_map(|b| b.as_u64().map(|n| n as u8))
                .collect(),
        )
        .unwrap_or_default(),
        _ => return None,
    };

    if body.is_empty() {
        return None;
    }

    let level = v
        .get("PRIORITY")
        .and_then(|p| p.as_str())
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(6);

    let ts_us = v
        .get("__REALTIME_TIMESTAMP")
        .and_then(|t| t.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or_else(now_micros);

    let unit = v
        .get("_SYSTEMD_UNIT")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string());

    Some(LogLine {
        source: source_name.to_string(),
        ts_us,
        level,
        body,
        unit,
    })
}

fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as i64
}
