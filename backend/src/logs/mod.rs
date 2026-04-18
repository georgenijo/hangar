pub mod file;
pub mod journalctl;
pub mod ringbuf_source;

use std::path::{Path, PathBuf};

use serde::Serialize;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::warn;

use crate::config::{LogSourceConfig, LogSourceKind, LogsConfig};

#[derive(Clone, Debug, Serialize)]
pub struct LogLine {
    pub source: String,
    pub ts_us: i64,
    pub level: u8,
    pub body: String,
    pub unit: Option<String>,
}

pub struct LogsHub {
    tx: broadcast::Sender<LogLine>,
    sources: Vec<LogSourceConfig>,
    ring_dir: PathBuf,
    pub tail_lines: usize,
    enabled: bool,
    tailer_handles: Vec<JoinHandle<()>>,
}

impl LogsHub {
    pub fn new(config: &LogsConfig, ring_dir: &Path) -> Self {
        let (tx, _) = broadcast::channel(4096);
        LogsHub {
            tx,
            sources: config.sources.clone(),
            ring_dir: ring_dir.to_path_buf(),
            tail_lines: config.tail_lines,
            enabled: config.enabled,
            tailer_handles: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        if !self.enabled {
            return;
        }
        for source in &self.sources {
            let tx = self.tx.clone();
            let tail_lines = self.tail_lines;
            let ring_dir = self.ring_dir.clone();
            match &source.kind {
                LogSourceKind::Journalctl => {
                    let tailer = journalctl::JournalctlTailer {
                        source_name: source.name.clone(),
                        unit: None,
                        tail_lines,
                    };
                    self.tailer_handles
                        .push(tokio::spawn(async move { tailer.run(tx).await }));
                }
                LogSourceKind::Unit => {
                    let tailer = journalctl::JournalctlTailer {
                        source_name: source.name.clone(),
                        unit: source.path.clone(),
                        tail_lines,
                    };
                    self.tailer_handles
                        .push(tokio::spawn(async move { tailer.run(tx).await }));
                }
                LogSourceKind::File => {
                    if let Some(path) = &source.path {
                        let tailer = file::FileTailer {
                            source_name: source.name.clone(),
                            path: PathBuf::from(path),
                            tail_lines,
                        };
                        self.tailer_handles
                            .push(tokio::spawn(async move { tailer.run(tx).await }));
                    } else {
                        warn!("file source '{}' missing path", source.name);
                    }
                }
                LogSourceKind::PaneScrollback => {
                    if let Some(session_id) = &source.session_id {
                        let tailer = ringbuf_source::RingbufTailer {
                            source_name: source.name.clone(),
                            session_id: session_id.clone(),
                            ring_dir,
                            tail_lines,
                        };
                        self.tailer_handles
                            .push(tokio::spawn(async move { tailer.run(tx).await }));
                    } else {
                        warn!(
                            "pane_scrollback source '{}' missing session_id",
                            source.name
                        );
                    }
                }
            }
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogLine> {
        self.tx.subscribe()
    }

    pub fn sources(&self) -> &[LogSourceConfig] {
        &self.sources
    }

    pub async fn initial_tail(&self, source_name: &str, n: usize) -> Vec<LogLine> {
        let source = match self.sources.iter().find(|s| s.name == source_name) {
            Some(s) => s,
            None => return Vec::new(),
        };
        match &source.kind {
            LogSourceKind::Journalctl => {
                let tailer = journalctl::JournalctlTailer {
                    source_name: source.name.clone(),
                    unit: None,
                    tail_lines: n,
                };
                tailer.initial_tail().await
            }
            LogSourceKind::Unit => {
                let tailer = journalctl::JournalctlTailer {
                    source_name: source.name.clone(),
                    unit: source.path.clone(),
                    tail_lines: n,
                };
                tailer.initial_tail().await
            }
            LogSourceKind::File => {
                if let Some(path) = &source.path {
                    let tailer = file::FileTailer {
                        source_name: source.name.clone(),
                        path: PathBuf::from(path),
                        tail_lines: n,
                    };
                    tailer.initial_tail().await
                } else {
                    Vec::new()
                }
            }
            LogSourceKind::PaneScrollback => {
                if let Some(session_id) = &source.session_id {
                    let tailer = ringbuf_source::RingbufTailer {
                        source_name: source.name.clone(),
                        session_id: session_id.clone(),
                        ring_dir: self.ring_dir.clone(),
                        tail_lines: n,
                    };
                    tailer.initial_tail().await
                } else {
                    Vec::new()
                }
            }
        }
    }
}
