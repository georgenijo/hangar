//! In-session worktree viewer endpoints (issue #48 v0).
//!
//! Read-only endpoints that expose the git state of a session's cwd so
//! the UI can render a VS-Code-style file tree + diff pane.
//!
//! * `GET /api/v1/sessions/:id/worktree/tree` — tracked + modified files
//!   with per-file status.
//! * `GET /api/v1/sessions/:id/worktree/file?path=<rel>` — file contents
//!   (trimmed to 500 KB).
//! * `GET /api/v1/sessions/:id/worktree/diff?path=<rel>` — unified diff
//!   vs HEAD for one file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::resolve_session;
use crate::AppState;

const MAX_FILE_BYTES: usize = 500 * 1024;
const MAX_DIFF_BYTES: usize = 1024 * 1024;

#[derive(Serialize)]
pub struct TreeEntry {
    pub path: String,
    pub status: FileStatus,
}

#[derive(Serialize)]
pub struct TreeResponse {
    pub root: String,
    pub branch: Option<String>,
    pub entries: Vec<TreeEntry>,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Clean,
    Modified,
    Added,
    Deleted,
    Untracked,
    Renamed,
}

#[derive(Deserialize)]
pub struct PathParams {
    pub path: String,
}

#[derive(Serialize)]
pub struct FileResponse {
    pub path: String,
    pub size: u64,
    pub truncated: bool,
    pub binary: bool,
    pub content: String,
}

#[derive(Serialize)]
pub struct DiffResponse {
    pub path: String,
    pub diff: String,
    pub truncated: bool,
}

/// Look up the session, return its cwd as a git worktree root or 404.
async fn session_worktree(state: &AppState, id: &str) -> Result<PathBuf, (StatusCode, String)> {
    let session = resolve_session(state, id)
        .await
        .map_err(|s| (s, "session not found".to_string()))?;
    let cwd = PathBuf::from(&session.cwd);
    if !cwd.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("session cwd {} does not exist", cwd.display()),
        ));
    }
    // Confirm it's inside a git worktree.
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(&cwd)
        .output();
    let top = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => {
            return Err((
                StatusCode::NOT_FOUND,
                "session cwd is not inside a git worktree".to_string(),
            ))
        }
    };
    Ok(PathBuf::from(top))
}

/// Resolve a user-supplied relative path against `root` and refuse any
/// path that escapes the worktree via `..` or symlinks.
fn safe_resolve(root: &Path, rel: &str) -> Result<PathBuf, (StatusCode, String)> {
    if rel.is_empty() || rel.starts_with('/') {
        return Err((StatusCode::BAD_REQUEST, "path must be relative".into()));
    }
    let joined = root.join(rel);
    // Canonicalize the parent; the file itself may not exist (deleted).
    let canonical_parent = match joined.parent() {
        Some(p) => std::fs::canonicalize(p)
            .map_err(|e| (StatusCode::NOT_FOUND, format!("bad path: {e}")))?,
        None => return Err((StatusCode::BAD_REQUEST, "bad path".into())),
    };
    let canonical_root = std::fs::canonicalize(root).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("canonicalize root: {e}"),
        )
    })?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err((StatusCode::FORBIDDEN, "path escapes worktree".into()));
    }
    Ok(canonical_parent.join(joined.file_name().unwrap_or_default()))
}

fn current_branch(root: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Parse `git status --porcelain=v1 -z` → map of path → status.
///
/// v1 is simpler to parse than v2 and gives us enough info for v0.
/// Format: `XY<space><path>\0` (renames add `<orig>\0` after).
fn status_map(root: &Path) -> Result<BTreeMap<String, FileStatus>, String> {
    let out = Command::new("git")
        .args(["status", "--porcelain=v1", "-z"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git status: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git status failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let mut map = BTreeMap::new();
    let mut iter = out.stdout.split(|b| *b == 0).peekable();
    while let Some(entry) = iter.next() {
        if entry.is_empty() {
            continue;
        }
        if entry.len() < 4 {
            continue;
        }
        let x = entry[0] as char;
        let y = entry[1] as char;
        // byte 2 is space
        let path = String::from_utf8_lossy(&entry[3..]).to_string();
        // For renames (R) the next chunk is the old path — consume it.
        if x == 'R' || y == 'R' {
            iter.next();
        }
        let status = match (x, y) {
            ('?', _) => FileStatus::Untracked,
            ('A', _) | (_, 'A') => FileStatus::Added,
            ('D', _) | (_, 'D') => FileStatus::Deleted,
            ('R', _) | (_, 'R') => FileStatus::Renamed,
            ('M', _) | (_, 'M') => FileStatus::Modified,
            _ => FileStatus::Modified,
        };
        map.insert(path, status);
    }
    Ok(map)
}

/// List tracked + untracked-non-ignored files via `git ls-files`.
fn tracked_files(root: &Path) -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git ls-files: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let files: Vec<String> = out
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).to_string())
        .collect();
    Ok(files)
}

pub async fn get_tree(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<TreeResponse>, (StatusCode, String)> {
    let root = session_worktree(&state, &id).await?;
    let branch = current_branch(&root);
    let statuses = status_map(&root).map_err(err500)?;
    let mut files = tracked_files(&root).map_err(err500)?;

    // Include deleted files (present in status but not in ls-files).
    for p in statuses.keys() {
        if !files.iter().any(|f| f == p) {
            files.push(p.clone());
        }
    }
    files.sort();
    files.dedup();

    let entries = files
        .into_iter()
        .map(|p| {
            let status = statuses.get(&p).copied().unwrap_or(FileStatus::Clean);
            TreeEntry { path: p, status }
        })
        .collect();

    Ok(Json(TreeResponse {
        root: root.display().to_string(),
        branch,
        entries,
    }))
}

pub async fn get_file(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Query(params): Query<PathParams>,
) -> Result<Json<FileResponse>, (StatusCode, String)> {
    let root = session_worktree(&state, &id).await?;
    let abs = safe_resolve(&root, &params.path)?;
    let bytes = std::fs::read(&abs)
        .map_err(|e| (StatusCode::NOT_FOUND, format!("read {}: {e}", params.path)))?;
    let total_len = bytes.len() as u64;
    let binary = bytes.iter().take(8192).any(|b| *b == 0);
    let (content, truncated) = if binary {
        (String::new(), false)
    } else if bytes.len() > MAX_FILE_BYTES {
        (
            String::from_utf8_lossy(&bytes[..MAX_FILE_BYTES]).to_string(),
            true,
        )
    } else {
        (String::from_utf8_lossy(&bytes).to_string(), false)
    };

    Ok(Json(FileResponse {
        path: params.path,
        size: total_len,
        truncated,
        binary,
        content,
    }))
}

pub async fn get_diff(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Query(params): Query<PathParams>,
) -> Result<Json<DiffResponse>, (StatusCode, String)> {
    let root = session_worktree(&state, &id).await?;
    // Validate the relative path (existence not required; file may be
    // deleted). We only care that it doesn't escape the worktree.
    if params.path.contains("..") || params.path.starts_with('/') {
        return Err((StatusCode::BAD_REQUEST, "bad path".into()));
    }

    // `git diff HEAD --` covers both staged and unstaged changes, and
    // includes untracked? No — untracked files aren't tracked. For
    // untracked, fall back to `git diff --no-index /dev/null <path>`.
    let out = Command::new("git")
        .args(["diff", "HEAD", "--", &params.path])
        .current_dir(&root)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("git diff: {e}")))?;

    let mut diff = String::from_utf8_lossy(&out.stdout).to_string();

    if diff.is_empty() {
        // Untracked file → synthesize an add-style diff.
        let abs = root.join(&params.path);
        if abs.exists() {
            let no_index = Command::new("git")
                .args(["diff", "--no-index", "--", "/dev/null", &params.path])
                .current_dir(&root)
                .output();
            if let Ok(o) = no_index {
                // no-index returns exit code 1 on difference; that's fine.
                diff = String::from_utf8_lossy(&o.stdout).to_string();
            }
        }
    }

    let truncated = diff.len() > MAX_DIFF_BYTES;
    if truncated {
        diff.truncate(MAX_DIFF_BYTES);
    }

    Ok(Json(DiffResponse {
        path: params.path,
        diff,
        truncated,
    }))
}

fn err500<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn run(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success(), "git {:?} failed", args);
    }

    fn init_repo() -> TempDir {
        let td = TempDir::new().unwrap();
        run(td.path(), &["init", "-q", "-b", "main"]);
        run(td.path(), &["config", "user.email", "t@t"]);
        run(td.path(), &["config", "user.name", "t"]);
        fs::write(td.path().join("a.txt"), "hello\n").unwrap();
        run(td.path(), &["add", "."]);
        run(td.path(), &["commit", "-qm", "init"]);
        td
    }

    #[test]
    fn status_map_reports_modified_and_untracked() {
        let td = init_repo();
        fs::write(td.path().join("a.txt"), "changed\n").unwrap();
        fs::write(td.path().join("new.txt"), "fresh\n").unwrap();
        let m = status_map(td.path()).unwrap();
        assert_eq!(m.get("a.txt").copied(), Some(FileStatus::Modified));
        assert_eq!(m.get("new.txt").copied(), Some(FileStatus::Untracked));
    }

    #[test]
    fn tracked_files_includes_untracked_non_ignored() {
        let td = init_repo();
        fs::write(td.path().join("b.txt"), "b\n").unwrap();
        let files = tracked_files(td.path()).unwrap();
        assert!(files.contains(&"a.txt".to_string()));
        assert!(files.contains(&"b.txt".to_string()));
    }

    #[test]
    fn safe_resolve_rejects_escape() {
        let td = init_repo();
        let root = td.path();
        // Parent (".." of the worktree root) is guaranteed to exist
        // because TempDir lives under /tmp, so canonicalize succeeds and
        // the prefix check fires with 403 rather than a 404.
        let err = safe_resolve(root, "../escape.txt").unwrap_err();
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    #[test]
    fn safe_resolve_rejects_absolute() {
        let td = init_repo();
        let err = safe_resolve(td.path(), "/etc/passwd").unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }
}
