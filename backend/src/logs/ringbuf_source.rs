use std::path::PathBuf;

use regex::Regex;
use tokio::sync::broadcast;
use tracing::warn;

use super::LogLine;
use crate::ringbuf::RingBuf;

pub struct RingbufTailer {
    pub source_name: String,
    pub session_id: String,
    pub ring_dir: PathBuf,
    pub tail_lines: usize,
}

impl RingbufTailer {
    fn ring_path(&self) -> PathBuf {
        self.ring_dir
            .join("sessions")
            .join(&self.session_id)
            .join("output.bin")
    }

    pub async fn initial_tail(&self) -> Vec<LogLine> {
        let path = self.ring_path();
        let ring = match RingBuf::open(&path) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        let head = ring.head();
        if head == 0 {
            return Vec::new();
        }

        let capacity = ring.capacity();
        let available = head.min(capacity);
        let start_offset = head.saturating_sub(capacity);

        let data = match ring.read_at(start_offset, available as u32) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        let strip_re = make_ansi_regex();
        let now = now_micros();
        let content = String::from_utf8_lossy(&data);
        let all: Vec<LogLine> = content
            .lines()
            .map(|l| strip_re.replace_all(l, "").to_string())
            .filter(|l| !l.trim().is_empty())
            .map(|l| LogLine {
                source: self.source_name.clone(),
                ts_us: now,
                level: 6,
                body: l.trim().to_string(),
                unit: None,
            })
            .collect();

        let skip = all.len().saturating_sub(self.tail_lines);
        all.into_iter().skip(skip).collect()
    }

    pub async fn run(self, tx: broadcast::Sender<LogLine>) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
        let path = self.ring_path();
        let mut last_head: u64 = 0;
        let strip_re = make_ansi_regex();

        loop {
            interval.tick().await;

            let ring = match RingBuf::open(&path) {
                Ok(r) => r,
                Err(_) => continue,
            };

            let head = ring.head();
            if head <= last_head {
                continue;
            }

            let capacity = ring.capacity();
            let unread = head - last_head;
            let to_read = unread.min(capacity) as u32;
            let read_offset = if unread > capacity {
                warn!(
                    "ringbuf source '{}' overrun, some lines lost",
                    self.source_name
                );
                head - capacity
            } else {
                last_head
            };

            match ring.read_at(read_offset, to_read) {
                Ok(data) => {
                    let now = now_micros();
                    let content = String::from_utf8_lossy(&data);
                    for line in content.lines() {
                        let stripped = strip_re.replace_all(line, "").to_string();
                        let trimmed = stripped.trim();
                        if !trimmed.is_empty() {
                            let _ = tx.send(LogLine {
                                source: self.source_name.clone(),
                                ts_us: now,
                                level: 6,
                                body: trimmed.to_string(),
                                unit: None,
                            });
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("ringbuf read error for '{}': {e}", self.source_name);
                }
            }

            last_head = head;
        }
    }
}

fn make_ansi_regex() -> Regex {
    Regex::new(r"\x1b(?:\[[0-9;?]*[A-Za-z]|.)").unwrap_or_else(|_| Regex::new(r"$^").unwrap())
}

fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as i64
}
