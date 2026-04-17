#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# Wave Pipeline Runner
# Plans all issues in a wave in parallel, builds sequentially
# Usage: wave.sh --project-dir /path --waves "15,16,17|12,21,22|32"
#   Pipe-separated waves, comma-separated issues within each wave
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/logging.sh"
source "$SCRIPT_DIR/lib/recovery.sh"

MAX_PARALLEL=4  # max concurrent planning sessions
PROJECT_DIR="$(pwd)"
WAVES=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --project-dir) PROJECT_DIR="$2"; shift 2 ;;
    --waves) WAVES="$2"; shift 2 ;;
    --max-parallel) MAX_PARALLEL="$2"; shift 2 ;;
    *) echo "Unknown arg: $1"; exit 1 ;;
  esac
done

if [ -z "$WAVES" ]; then
  echo "Usage: wave.sh --project-dir /path --waves '15,16,17|12,21,22|32'"
  exit 1
fi

PROJECT_NAME="$(basename "$PROJECT_DIR")"
CLAUDE_BIN="${CLAUDE_BIN:-claude}"
AGENTS_DIR="$SCRIPT_DIR/agents"
LOGS_ROOT="$HOME/Documents/pipeline-logs/$PROJECT_NAME"

REPO_SLUG=$(git -C "$PROJECT_DIR" remote get-url origin | sed 's|.*github.com[:/]||;s|\.git$||')

# --- Planning function (must be defined before use) ---
_plan_single_issue() {
  local issue="$1"
  local log_dir="$LOGS_ROOT/issue-$issue"
  local log_file="$log_dir/pipeline.json"

  init_log "$PROJECT_NAME" "$issue"

  local issue_body
  issue_body=$(gh issue view "$issue" --repo "$REPO_SLUG" --json title,body -q '"# Issue #\(.number // ""): \(.title)\n\n\(.body)"' 2>/dev/null || echo "Failed to fetch issue")

  local context_file="$log_dir/context.md"
  log_agent_start "context-gatherer" 1

  $CLAUDE_BIN -p "Gather context for this issue in the project at $PROJECT_DIR.

Issue:
$issue_body

Write your context document to: $context_file" \
    --dangerously-skip-permissions \
    --model sonnet \
    --system-prompt-file "$AGENTS_DIR/context-gatherer.md" \
    --add-dir "$PROJECT_DIR" \
    2>"$log_dir/context-gatherer-stderr.log" || { log_agent_end "context-gatherer" 1; return 1; }

  log_agent_end "context-gatherer" 0
  [ -f "$context_file" ] || { echo "No context.md"; return 1; }

  local context_content plan_file review_file plan_version=1 approved=false round=0
  context_content="$(cat "$context_file")"

  while [ "$approved" = false ] && [ "$round" -lt 3 ]; do
    round=$((round + 1))

    local feedback=""
    local prev_review="$log_dir/review-$((round - 1)).md"
    [ -f "$prev_review" ] && feedback="

## Reviewer Feedback
$(cat "$prev_review")"

    plan_file="$log_dir/plan-v${plan_version}.md"
    log_agent_start "architect" "$round"

    $CLAUDE_BIN -p "Create an implementation plan for this issue.

$context_content
$feedback

Write your plan to: $plan_file" \
      --dangerously-skip-permissions \
      --model opus \
      --system-prompt-file "$AGENTS_DIR/architect.md" \
      --add-dir "$PROJECT_DIR" \
      2>"$log_dir/architect-${round}-stderr.log" || { log_agent_end "architect" 1; return 1; }

    log_agent_end "architect" 0
    [ -f "$plan_file" ] || { echo "No plan"; return 1; }

    review_file="$log_dir/review-${round}.md"
    local plan_content
    plan_content="$(cat "$plan_file")"

    log_agent_start "senior-reviewer" "$round"

    $CLAUDE_BIN -p "Review this implementation plan.

## Context
$context_content

## Plan
$plan_content

Write your review to: $review_file" \
      --dangerously-skip-permissions \
      --model opus \
      --system-prompt-file "$AGENTS_DIR/senior-reviewer.md" \
      --add-dir "$PROJECT_DIR" \
      2>"$log_dir/reviewer-${round}-stderr.log" || { log_agent_end "senior-reviewer" 1; return 1; }

    log_agent_end "senior-reviewer" 0

    if grep -qi "^# Review: APPROVED" "$review_file"; then
      approved=true
      tmp=$(mktemp)
      jq '.completed_steps += ["architect-approved"]' "$log_file" > "$tmp" && mv "$tmp" "$log_file"
    else
      plan_version=$((plan_version + 1))
    fi
  done

  [ "$approved" = true ] || return 1
}

echo "=== Wave Pipeline: $PROJECT_NAME ==="
echo "Project:      $PROJECT_DIR"
echo "Max parallel: $MAX_PARALLEL"
echo ""

# Parse waves
IFS='|' read -ra WAVE_LIST <<< "$WAVES"
WAVE_NUM=0

for wave in "${WAVE_LIST[@]}"; do
  WAVE_NUM=$((WAVE_NUM + 1))
  IFS=',' read -ra ISSUES <<< "$wave"

  echo ""
  echo "################################################################"
  echo "# WAVE $WAVE_NUM: issues ${ISSUES[*]}"
  echo "################################################################"

  # --- Phase 1: Parallel Planning ---
  echo ""
  echo "=== Phase 1: Planning (parallel, max $MAX_PARALLEL) ==="

  PLAN_PIDS=()
  PLAN_ISSUES=()
  RUNNING=0

  for issue in "${ISSUES[@]}"; do
    issue=$(echo "$issue" | tr -d ' ')
    ISSUE_LOG_DIR="$LOGS_ROOT/issue-$issue"
    mkdir -p "$ISSUE_LOG_DIR"

    # Skip if already planned
    if [ -f "$ISSUE_LOG_DIR/pipeline.json" ] && jq -e '.completed_steps | index("architect-approved")' "$ISSUE_LOG_DIR/pipeline.json" >/dev/null 2>&1; then
      echo "  #$issue — plan already approved, skipping"
      continue
    fi

    # Throttle
    while [ "$RUNNING" -ge "$MAX_PARALLEL" ]; do
      # Wait for any child to finish
      wait -n 2>/dev/null || true
      RUNNING=$((RUNNING - 1))
    done

    echo "  #$issue — planning..."

    # Run planning phase in background
    (
      _plan_single_issue "$issue"
    ) > "$ISSUE_LOG_DIR/plan-phase.log" 2>&1 &

    PLAN_PIDS+=($!)
    PLAN_ISSUES+=("$issue")
    RUNNING=$((RUNNING + 1))
  done

  # Wait for all planning to finish
  echo "  Waiting for ${#PLAN_PIDS[@]} planning sessions..."
  PLAN_FAILURES=0
  for i in "${!PLAN_PIDS[@]}"; do
    if wait "${PLAN_PIDS[$i]}"; then
      echo "  ✓ #${PLAN_ISSUES[$i]} — plan approved"
    else
      echo "  ✗ #${PLAN_ISSUES[$i]} — planning failed (see logs)"
      PLAN_FAILURES=$((PLAN_FAILURES + 1))
    fi
  done

  if [ "$PLAN_FAILURES" -gt 0 ]; then
    echo "  ⚠ $PLAN_FAILURES planning failures. Continuing with successful ones."
  fi

  # --- Phase 2: Sequential Build + Test + Merge ---
  echo ""
  echo "=== Phase 2: Build + Test + Merge (sequential) ==="

  for issue in "${ISSUES[@]}"; do
    issue=$(echo "$issue" | tr -d ' ')
    ISSUE_LOG_DIR="$LOGS_ROOT/issue-$issue"

    # Skip if no approved plan
    if [ ! -f "$ISSUE_LOG_DIR/pipeline.json" ] || ! jq -e '.completed_steps | index("architect-approved")' "$ISSUE_LOG_DIR/pipeline.json" >/dev/null 2>&1; then
      echo "  #$issue — no approved plan, skipping build"
      continue
    fi

    # Skip if already complete
    STATUS=$(jq -r '.status // "unknown"' "$ISSUE_LOG_DIR/pipeline.json")
    if [ "$STATUS" = "completed" ]; then
      echo "  #$issue — already completed, skipping"
      continue
    fi

    echo ""
    echo "  --- Building #$issue ---"

    # Pull latest main before each build
    git -C "$PROJECT_DIR" checkout main 2>/dev/null
    git -C "$PROJECT_DIR" pull --rebase 2>/dev/null || true

    # Run full pipeline (it will skip planning phases via checkpoint)
    if "$SCRIPT_DIR/pipeline.sh" "$issue" --project-dir "$PROJECT_DIR"; then
      echo "  ✓ #$issue — merged"
    else
      echo "  ✗ #$issue — build/test failed (see logs)"
    fi
  done

  echo ""
  echo "=== Wave $WAVE_NUM complete ==="
done

echo ""
echo "=== All waves complete ==="
