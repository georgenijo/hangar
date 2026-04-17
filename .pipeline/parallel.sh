#!/usr/bin/env bash
# ============================================================
# Parallel Pipeline Runner
#
# Runs multiple hangar pipelines concurrently, each in its own
# git worktree + tmux session. Branches stay isolated; conflicts
# surface only at PR/merge time.
#
# Usage:
#   parallel.sh --project-dir /path/to/repo \
#               --issues 1,6,7,8,9 \
#               [--concurrent 3]
# ============================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PROJECT_DIR="$(pwd)"
ISSUES=""
CONCURRENT=3
STAGGER_SECONDS=30
DRY_RUN=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --project-dir)  PROJECT_DIR="$(cd "$2" && pwd)"; shift 2 ;;
    --issues)       ISSUES="$2"; shift 2 ;;
    --concurrent)   CONCURRENT="$2"; shift 2 ;;
    --stagger)      STAGGER_SECONDS="$2"; shift 2 ;;
    --dry-run)      DRY_RUN=true; shift ;;
    *) echo "Unknown arg: $1"; exit 1 ;;
  esac
done

if [[ -z "$ISSUES" ]]; then
  echo "Usage: parallel.sh --project-dir /path --issues 1,6,7 [--concurrent 3]"
  exit 1
fi

PROJECT_NAME="$(basename "$PROJECT_DIR")"
WORKTREE_ROOT="$(dirname "$PROJECT_DIR")/${PROJECT_NAME}.worktrees"
LOGS_DIR="$HOME/Documents/pipeline-logs/$PROJECT_NAME"
TMUX_PREFIX="hangar-pipe"

if [[ "$DRY_RUN" == false ]]; then
  mkdir -p "$WORKTREE_ROOT" "$LOGS_DIR"
fi

IFS=',' read -ra QUEUE <<< "$ISSUES"

echo "=== Parallel Pipeline ==="
echo "Project:        $PROJECT_DIR ($PROJECT_NAME)"
echo "Worktree root:  $WORKTREE_ROOT"
echo "Queue:          ${QUEUE[*]}"
echo "Max concurrent: $CONCURRENT"
echo "Stagger:        ${STAGGER_SECONDS}s"
echo "Logs:           $LOGS_DIR"
echo

# Running tmux sessions matching our prefix
running_count() {
  tmux ls 2>/dev/null | grep -c "^${TMUX_PREFIX}-" || true
}
running_sessions() {
  tmux ls 2>/dev/null | grep "^${TMUX_PREFIX}-" | cut -d: -f1 | tr '\n' ' '
}

dispatch_one() {
  local issue="$1"
  local wt="$WORKTREE_ROOT/issue-$issue"
  local session="${TMUX_PREFIX}-$issue"
  local pipe_log="$LOGS_DIR/parallel-$issue.log"

  if [[ "$DRY_RUN" == true ]]; then
    echo "  [DRY] issue #$issue would:"
    echo "        git worktree add --detach '$wt' origin/main"
    echo "        tmux new-session -d -s '$session' ..."
    echo "        invoke '$SCRIPT_DIR/pipeline.sh' '$issue' --project-dir '$wt'"
    echo "        log -> $pipe_log"
    return
  fi

  # Already running?
  if tmux has-session -t "$session" 2>/dev/null; then
    echo "  [issue #$issue] session already exists; skipping dispatch"
    return
  fi

  # Create worktree from main if it doesn't exist. Detached HEAD so pipeline.sh
  # can create its own issue branch without a preexisting branch claiming main.
  if [[ ! -d "$wt" ]]; then
    echo "  [issue #$issue] creating worktree at $wt"
    git -C "$PROJECT_DIR" fetch -q origin main || true
    git -C "$PROJECT_DIR" worktree add --detach "$wt" origin/main >/dev/null 2>&1 \
      || git -C "$PROJECT_DIR" worktree add --detach "$wt" main
  else
    echo "  [issue #$issue] reusing existing worktree at $wt"
  fi

  # Spawn pipeline in its own tmux session, inside the worktree.
  # PROJECT_NAME_OVERRIDE keeps logs unified under $PROJECT_NAME even though
  # pipeline.sh would otherwise derive a different name from basename(PROJECT_DIR).
  tmux new-session -d -s "$session" \
    "cd '$wt' && source ~/.cargo/env 2>/dev/null || true; \
     export PROJECT_NAME_OVERRIDE='$PROJECT_NAME'; \
     '$SCRIPT_DIR/pipeline.sh' '$issue' --project-dir '$wt' 2>&1 \
       | tee '$pipe_log'; \
     echo; echo '=== pipeline exited for issue $issue ==='; exec bash"

  echo "  [issue #$issue] spawned tmux session $session (log: $pipe_log)"
}

# Main scheduling loop
for issue in "${QUEUE[@]}"; do
  if [[ "$DRY_RUN" == false ]]; then
    while [[ "$(running_count)" -ge "$CONCURRENT" ]]; do
      echo "  throttled: $(running_count) running ($(running_sessions))"
      sleep 10
    done
  fi
  dispatch_one "$issue"
  if [[ "$DRY_RUN" == false && "$issue" != "${QUEUE[-1]}" ]]; then
    sleep "$STAGGER_SECONDS"
  fi
done

if [[ "$DRY_RUN" == true ]]; then
  echo
  echo "=== DRY RUN — nothing was spawned or modified ==="
  exit 0
fi

echo
echo "All $((${#QUEUE[@]})) issues dispatched. Waiting for completion..."
while [[ "$(running_count)" -gt 0 ]]; do
  sleep 30
  echo "  $(date +%H:%M:%S) — $(running_count) running: $(running_sessions)"
done

echo
echo "=== Parallel batch complete ==="
echo
echo "Worktree branches to review / merge:"
for issue in "${QUEUE[@]}"; do
  wt="$WORKTREE_ROOT/issue-$issue"
  if [[ -d "$wt" ]]; then
    branch=$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "(none)")
    commits=$(git -C "$wt" log origin/main..HEAD --oneline 2>/dev/null | wc -l | tr -d ' ')
    echo "  issue #$issue  branch=$branch  commits_ahead_of_main=$commits  worktree=$wt"
  fi
done

echo
echo "Cleanup (after branches merged / abandoned):"
echo "  for i in ${QUEUE[*]}; do git -C '$PROJECT_DIR' worktree remove '$WORKTREE_ROOT'/issue-\$i; done"
