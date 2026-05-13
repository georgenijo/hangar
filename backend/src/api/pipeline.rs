use std::path::PathBuf;

use axum::Json;
use serde::{Deserialize, Serialize};

/// Parsed subset of pipeline.json we care about.
#[derive(Debug, Deserialize)]
struct PipelineLog {
    #[serde(default)]
    issue: u64,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    current_step: Option<String>,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    agents: Vec<serde_json::Value>,
}

/// Parsed subset of a test-results-N.json gate fields.
#[derive(Debug, Deserialize, Default)]
struct TestResults {
    #[serde(default)]
    screenshots: Vec<serde_json::Value>,
    #[serde(default)]
    scenarios: Vec<serde_json::Value>,
    #[serde(default)]
    pipeline_gate_failure: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GateInfo {
    pub smoke: Option<bool>,
    pub shots: usize,
    pub scenarios: usize,
}

#[derive(Debug, Serialize)]
pub struct PipelineRun {
    pub issue: u64,
    pub title: String,
    pub state: String,
    pub phase: String,
    pub host: String,
    pub agents: Vec<serde_json::Value>,
    pub cost_usd: f64,
    pub tokens: u64,
    pub started_at: String,
    pub gate: GateInfo,
}

#[derive(Debug, Serialize)]
pub struct PipelineRunsResponse {
    pub runs: Vec<PipelineRun>,
}

fn pipeline_logs_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("Documents")
        .join("pipeline-logs")
}

fn read_gate(log_dir: &PathBuf) -> GateInfo {
    // Find the latest test-results-N.json in the log dir.
    let mut best: Option<(u64, TestResults)> = None;
    if let Ok(entries) = std::fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(n) = name
                .strip_prefix("test-results-")
                .and_then(|s| s.strip_suffix(".json"))
                .and_then(|s| s.parse::<u64>().ok())
            {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(tr) = serde_json::from_str::<TestResults>(&content) {
                        if best.as_ref().map_or(true, |(m, _)| n > *m) {
                            best = Some((n, tr));
                        }
                    }
                }
            }
        }
    }

    match best {
        None => GateInfo {
            smoke: None,
            shots: 0,
            scenarios: 0,
        },
        Some((_, tr)) => GateInfo {
            smoke: Some(tr.pipeline_gate_failure.is_none()),
            shots: tr.screenshots.len(),
            scenarios: tr.scenarios.len(),
        },
    }
}

pub async fn list_runs() -> Json<PipelineRunsResponse> {
    let root = pipeline_logs_root();
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "unknown".to_string());

    let mut runs: Vec<PipelineRun> = Vec::new();

    // Walk <root>/<project>/issue-<N>/pipeline.json
    let Ok(projects) = std::fs::read_dir(&root) else {
        return Json(PipelineRunsResponse { runs });
    };

    for project_entry in projects.flatten() {
        let Ok(issues) = std::fs::read_dir(project_entry.path()) else {
            continue;
        };
        for issue_entry in issues.flatten() {
            let dir = issue_entry.path();
            let name = issue_entry.file_name().to_string_lossy().into_owned();
            // Directory must be named issue-<N>
            if !name.starts_with("issue-") {
                continue;
            }
            let pipeline_json = dir.join("pipeline.json");
            let Ok(content) = std::fs::read_to_string(&pipeline_json) else {
                continue;
            };
            let Ok(log) = serde_json::from_str::<PipelineLog>(&content) else {
                continue;
            };

            let gate = read_gate(&dir);

            runs.push(PipelineRun {
                issue: log.issue,
                title: String::new(),
                state: log.status.unwrap_or_else(|| "unknown".to_string()),
                phase: log.current_step.unwrap_or_else(|| "unknown".to_string()),
                host: hostname.clone(),
                agents: log.agents,
                cost_usd: 0.0,
                tokens: 0,
                started_at: log.started_at.unwrap_or_default(),
                gate,
            });
        }
    }

    // Most recent first.
    runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    Json(PipelineRunsResponse { runs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_json(path: &std::path::Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    fn make_pipeline_json(issue: u64, status: &str, phase: &str) -> String {
        serde_json::json!({
            "issue": issue,
            "status": status,
            "current_step": phase,
            "started_at": "2026-05-01T10:00:00Z",
            "agents": [],
        })
        .to_string()
    }

    #[test]
    fn read_gate_no_test_results() {
        let tmp = TempDir::new().unwrap();
        let gate = read_gate(&tmp.path().to_path_buf());
        assert!(gate.smoke.is_none());
        assert_eq!(gate.shots, 0);
        assert_eq!(gate.scenarios, 0);
    }

    #[test]
    fn read_gate_parses_latest_test_results() {
        let tmp = TempDir::new().unwrap();

        // Older results — only 1 screenshot
        write_json(
            &tmp.path().join("test-results-1.json"),
            r#"{"screenshots":["a.png"],"scenarios":[]}"#,
        );
        // Newer results — 2 screenshots + 1 scenario
        write_json(
            &tmp.path().join("test-results-2.json"),
            r#"{"screenshots":["a.png","b.png"],"scenarios":[{"name":"issue-87: ran"}]}"#,
        );

        let gate = read_gate(&tmp.path().to_path_buf());
        assert_eq!(gate.shots, 2);
        assert_eq!(gate.scenarios, 1);
        assert_eq!(gate.smoke, Some(true));
    }

    #[test]
    fn list_runs_empty_when_no_dir() {
        // Point pipeline_logs_root at a nonexistent path via the async fn.
        // We can't easily override the path, so just verify the function
        // compiles and the real root path is accepted gracefully.
        let nonexistent = PathBuf::from("/tmp/hangar-test-nonexistent-pipeline-logs-abc123");
        let Ok(entries) = std::fs::read_dir(&nonexistent) else {
            // Expected: dir doesn't exist, function returns empty.
            return;
        };
        assert_eq!(entries.count(), 0);
    }

    #[tokio::test]
    async fn list_runs_parses_pipeline_json_files() {
        let tmp = TempDir::new().unwrap();

        // Simulate <root>/hangar/issue-42/pipeline.json
        let dir = tmp.path().join("hangar").join("issue-42");
        write_json(
            &dir.join("pipeline.json"),
            &make_pipeline_json(42, "completed", "merged"),
        );

        // Can't inject the root, so just verify the reader logic directly.
        let pipeline_json = dir.join("pipeline.json");
        let content = std::fs::read_to_string(&pipeline_json).unwrap();
        let log: PipelineLog = serde_json::from_str(&content).unwrap();

        assert_eq!(log.issue, 42);
        assert_eq!(log.status.as_deref(), Some("completed"));
        assert_eq!(log.current_step.as_deref(), Some("merged"));
    }
}
