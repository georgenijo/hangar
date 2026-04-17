use axum::{extract::State, Json};
use serde::Serialize;

use crate::config::LogSourceKind;
use crate::AppState;

#[derive(Serialize)]
pub struct SourceInfo {
    pub name: String,
    pub kind: String,
    pub active: bool,
}

pub async fn list_sources(State(state): State<AppState>) -> Json<Vec<SourceInfo>> {
    let sources = state
        .logs
        .sources()
        .iter()
        .map(|s| SourceInfo {
            name: s.name.clone(),
            kind: match s.kind {
                LogSourceKind::Journalctl => "journalctl",
                LogSourceKind::Unit => "unit",
                LogSourceKind::File => "file",
                LogSourceKind::PaneScrollback => "pane_scrollback",
            }
            .to_string(),
            active: true,
        })
        .collect();
    Json(sources)
}
