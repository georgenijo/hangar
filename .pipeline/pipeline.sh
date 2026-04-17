#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# Claude Agent Pipeline
# Usage: pipeline.sh <issue-number> [--project-dir /path/to/repo]
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/logging.sh"
source "$SCRIPT_DIR/lib/recovery.sh"

# --- Config ---
MAX_REVIEW_ROUNDS=3
MAX_FIX_ROUNDS=3
CLAUDE_BIN="${CLAUDE_BIN:-claude}"
AGENTS_DIR="$SCRIPT_DIR/agents"

# --- Args ---
ISSUE_NUM="${1:?Usage: pipeline.sh <issue-number> [--project-dir /path]}"
shift

PROJECT_DIR="$(pwd)"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --project-dir) PROJECT_DIR="$2"; shift 2 ;;
    *) echo "Unknown arg: $1"; exit 1 ;;
  esac
done

PROJECT_NAME="$(basename "$PROJECT_DIR")"
LOGS_DIR="$HOME/Documents/pipeline-logs/$PROJECT_NAME/issue-$ISSUE_NUM"

# --- Init ---
echo "=== Pipeline: $PROJECT_NAME #$ISSUE_NUM ==="
echo "Project: $PROJECT_DIR"
echo "Logs:    $LOGS_DIR"

init_log "$PROJECT_NAME" "$ISSUE_NUM"

# Check for crash recovery
status=$(get_pipeline_status "$LOG_FILE")
if [ "$status" = "running" ]; then
  echo "⚠ Previous run was interrupted. Resuming..."
  mark_resumed "$LOG_FILE"
fi

# Fetch issue body
ISSUE_BODY=$(gh issue view "$ISSUE_NUM" --repo "$(git -C "$PROJECT_DIR" remote get-url origin | sed 's|.*github.com[:/]||;s|\.git$||')" --json title,body -q '"# Issue #\(.number // ""): \(.title)\n\n\(.body)"' 2>/dev/null || echo "Failed to fetch issue")

# --- Helper: run an agent ---
run_agent() {
  local agent_name="$1"
  local model="$2"
  local prompt="$3"
  local iteration="${4:-1}"

  echo ""
  echo "--- [$agent_name] (model=$model, iter=$iteration) ---"

  log_agent_start "$agent_name" "$iteration" "$model"

  local start_time=$SECONDS
  local exit_code=0

  $CLAUDE_BIN -p "$prompt" \
    --dangerously-skip-permissions \
    --model "$model" \
    --system-prompt-file "$AGENTS_DIR/$agent_name.md" \
    --add-dir "$PROJECT_DIR" \
    2>"$LOGS_DIR/${agent_name}-${iteration}-stderr.log" \
    || exit_code=$?

  local duration=$(( SECONDS - start_time ))
  echo "    Duration: ${duration}s, exit: $exit_code"

  log_agent_end "$agent_name" "$exit_code"

  return $exit_code
}

# --- Step 1: Context Gathering ---
if ! should_skip_step "$LOG_FILE" "context-gatherer"; then
  echo ""
  echo "========== STEP 1: Context Gathering =========="

  CONTEXT_FILE="$LOGS_DIR/context.md"

  MODELS_FILE="$LOGS_DIR/models.json"

  run_agent "context-gatherer" "sonnet" \
    "Gather context for this issue in the project at $PROJECT_DIR.

Issue:
$ISSUE_BODY

Write your context document to: $CONTEXT_FILE
Write your model assignments to: $MODELS_FILE"

  log_step_artifact "$CONTEXT_FILE"
  log_step_artifact "$MODELS_FILE"

  if [ ! -f "$CONTEXT_FILE" ]; then
    log_error "Context gatherer did not produce context.md"
    echo "ERROR: context.md not created"
    exit 1
  fi
else
  echo "Skipping context-gatherer (already completed)"
  CONTEXT_FILE="$LOGS_DIR/context.md"
fi

# --- Load Model Assignments ---
MODELS_FILE="$LOGS_DIR/models.json"

# Defaults
MODEL_ARCHITECT="opus"
MODEL_REVIEWER="opus"
MODEL_BUILDER="sonnet"
MODEL_TESTER="sonnet"
MODEL_FIXER="sonnet"

if [ -f "$MODELS_FILE" ]; then
  MODEL_ARCHITECT=$(jq -r '.assignments.architect // "opus"' "$MODELS_FILE")
  MODEL_REVIEWER=$(jq -r '.assignments["senior-reviewer"] // "opus"' "$MODELS_FILE")
  MODEL_BUILDER=$(jq -r '.assignments.builder // "sonnet"' "$MODELS_FILE")
  MODEL_TESTER=$(jq -r '.assignments.tester // "sonnet"' "$MODELS_FILE")
  MODEL_FIXER=$(jq -r '.assignments.fixer // "sonnet"' "$MODELS_FILE")
  COMPLEXITY=$(jq -r '.complexity // "unknown"' "$MODELS_FILE")

  log_model_assignments "$MODELS_FILE"

  echo ""
  echo "Model assignments (complexity=$COMPLEXITY):"
  echo "  architect=$MODEL_ARCHITECT reviewer=$MODEL_REVIEWER builder=$MODEL_BUILDER tester=$MODEL_TESTER fixer=$MODEL_FIXER"
else
  echo "No models.json — using defaults (opus/sonnet)"
fi

# --- Step 2 & 3: Architect + Review Loop ---
PLAN_VERSION=1
PLAN_FILE="$LOGS_DIR/plan.md"
APPROVED=false
REVIEW_ROUND=0

if ! should_skip_step "$LOG_FILE" "architect-approved"; then
  echo ""
  echo "========== STEP 2-3: Architecture + Review =========="

  REVIEW_ROUND=$(get_review_iteration "$LOG_FILE")
  CONTEXT_CONTENT="$(cat "$CONTEXT_FILE")"

  while [ "$APPROVED" = false ] && [ "$REVIEW_ROUND" -lt "$MAX_REVIEW_ROUNDS" ]; do
    REVIEW_ROUND=$((REVIEW_ROUND + 1))

    # Build feedback context for re-plans
    FEEDBACK=""
    PREV_REVIEW="$LOGS_DIR/review-$((REVIEW_ROUND - 1)).md"
    if [ -f "$PREV_REVIEW" ]; then
      FEEDBACK="

## Reviewer Feedback (round $((REVIEW_ROUND - 1)))
$(cat "$PREV_REVIEW")"
    fi

    # Architect
    PLAN_FILE="$LOGS_DIR/plan-v${PLAN_VERSION}.md"
    run_agent "architect" "$MODEL_ARCHITECT" \
      "Create an implementation plan for this issue.

$CONTEXT_CONTENT
$FEEDBACK

Write your plan to: $PLAN_FILE" "$REVIEW_ROUND"

    log_step_artifact "$PLAN_FILE"

    if [ ! -f "$PLAN_FILE" ]; then
      log_error "Architect did not produce plan"
      echo "ERROR: Plan not created"
      exit 1
    fi

    # Senior Reviewer
    REVIEW_FILE="$LOGS_DIR/review-${REVIEW_ROUND}.md"
    PLAN_CONTENT="$(cat "$PLAN_FILE")"

    run_agent "senior-reviewer" "$MODEL_REVIEWER" \
      "Review this implementation plan.

## Context
$CONTEXT_CONTENT

## Plan
$PLAN_CONTENT

Write your review to: $REVIEW_FILE" "$REVIEW_ROUND"

    log_step_artifact "$REVIEW_FILE"

    if [ ! -f "$REVIEW_FILE" ]; then
      log_error "Reviewer did not produce review"
      echo "ERROR: Review not created"
      exit 1
    fi

    # Check if approved
    if grep -qi "^# Review: APPROVED" "$REVIEW_FILE"; then
      APPROVED=true
      log_review_result "$REVIEW_ROUND" true
      echo "    ✓ Plan APPROVED (round $REVIEW_ROUND)"

      # Mark architect-approved as completed step
      tmp=$(mktemp)
      jq '.completed_steps += ["architect-approved"]' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
    else
      echo "    ✗ Plan NEEDS REVISION (round $REVIEW_ROUND/$MAX_REVIEW_ROUNDS)"
      log_review_result "$REVIEW_ROUND" false
      PLAN_VERSION=$((PLAN_VERSION + 1))
    fi
  done

  if [ "$APPROVED" = false ]; then
    log_error "Plan not approved after $MAX_REVIEW_ROUNDS rounds"
    echo "ERROR: Plan not approved after $MAX_REVIEW_ROUNDS review rounds. Manual intervention needed."
    exit 1
  fi
else
  echo "Skipping architect+review (already approved)"
  # Find latest approved plan
  PLAN_FILE=$(ls -t "$LOGS_DIR"/plan-v*.md 2>/dev/null | head -1)
fi

# --- Step 4: Build ---
if ! should_skip_step "$LOG_FILE" "builder"; then
  echo ""
  echo "========== STEP 4: Building =========="

  PLAN_CONTENT="$(cat "$PLAN_FILE")"
  CONTEXT_CONTENT="$(cat "$CONTEXT_FILE")"

  # Create branch
  BRANCH_NAME="issue-${ISSUE_NUM}-$(echo "$ISSUE_BODY" | head -1 | sed 's/[^a-zA-Z0-9]/-/g' | tr '[:upper:]' '[:lower:]' | cut -c1-40)"
  git -C "$PROJECT_DIR" checkout -b "$BRANCH_NAME" 2>/dev/null || git -C "$PROJECT_DIR" checkout "$BRANCH_NAME"

  run_agent "builder" "$MODEL_BUILDER" \
    "Implement the following approved plan in the project at $PROJECT_DIR.
You are on branch $BRANCH_NAME.

## Context
$CONTEXT_CONTENT

## Approved Plan
$PLAN_CONTENT

Implement all changes. Commit when done."

  log_step_artifact "branch:$BRANCH_NAME"

  # Update log with branch
  tmp=$(mktemp)
  jq --arg branch "$BRANCH_NAME" '.branch = $branch' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
else
  echo "Skipping builder (already completed)"
  BRANCH_NAME=$(jq -r '.branch // ""' "$LOG_FILE")
fi

# --- Step 5 & 6: Test + Fix Loop ---
FIX_ROUND=0
TESTS_PASS=false

if ! should_skip_step "$LOG_FILE" "tests-pass"; then
  echo ""
  echo "========== STEP 5-6: Testing + Fixing =========="

  FIX_ROUND=$(get_fix_iteration "$LOG_FILE")
  PLAN_CONTENT="$(cat "$PLAN_FILE")"

  while [ "$TESTS_PASS" = false ] && [ "$FIX_ROUND" -lt "$MAX_FIX_ROUNDS" ]; do
    FIX_ROUND=$((FIX_ROUND + 1))

    # Test
    TEST_RESULTS="$LOGS_DIR/test-results-${FIX_ROUND}.json"

    run_agent "tester" "$MODEL_TESTER" \
      "Test the changes on branch $BRANCH_NAME in $PROJECT_DIR.

## Plan (what was built)
$PLAN_CONTENT

Write test results to: $TEST_RESULTS" "$FIX_ROUND"

    log_step_artifact "$TEST_RESULTS"

    if [ ! -f "$TEST_RESULTS" ]; then
      echo "    ⚠ No test results file — assuming needs fixing"
    elif jq -e '.status == "PASS"' "$TEST_RESULTS" >/dev/null 2>&1; then
      TESTS_PASS=true
      log_test_result "$FIX_ROUND" true
      echo "    ✓ Tests PASS (round $FIX_ROUND)"

      tmp=$(mktemp)
      jq '.completed_steps += ["tests-pass"]' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
      continue
    else
      log_test_result "$FIX_ROUND" false
      echo "    ✗ Tests FAIL (round $FIX_ROUND/$MAX_FIX_ROUNDS)"
    fi

    # Fix
    if [ "$TESTS_PASS" = false ] && [ "$FIX_ROUND" -lt "$MAX_FIX_ROUNDS" ]; then
      TEST_CONTENT="$(cat "$TEST_RESULTS" 2>/dev/null || echo 'No test results file')"

      run_agent "fixer" "$MODEL_FIXER" \
        "Fix the failing tests in $PROJECT_DIR on branch $BRANCH_NAME.

## Test Results
$TEST_CONTENT

## Original Plan
$PLAN_CONTENT

Fix the bugs and commit." "$FIX_ROUND"

      log_step_artifact "fix-round-$FIX_ROUND"
    fi
  done
fi

# --- Step 7: PR + Merge ---
echo ""
echo "========== STEP 7: PR + Merge =========="

REPO_SLUG=$(git -C "$PROJECT_DIR" remote get-url origin | sed 's|.*github.com[:/]||;s|\.git$||')

# Push branch
git -C "$PROJECT_DIR" push -u origin "$BRANCH_NAME" 2>/dev/null || true

# Create PR
ISSUE_TITLE=$(gh issue view "$ISSUE_NUM" --repo "$REPO_SLUG" --json title -q '.title' 2>/dev/null || echo "Issue $ISSUE_NUM")

if [ "$TESTS_PASS" = true ]; then
  PR_URL=$(gh pr create \
    --repo "$REPO_SLUG" \
    --head "$BRANCH_NAME" \
    --title "feat: $ISSUE_TITLE" \
    --body "$(cat <<PREOF
Closes #$ISSUE_NUM

## Pipeline Run
- Plan: $(basename "$PLAN_FILE")
- Review rounds: $REVIEW_ROUND
- Test rounds: $FIX_ROUND
- Status: All tests passing

Logs: \`pipeline-logs/$PROJECT_NAME/issue-$ISSUE_NUM/\`
PREOF
)" 2>/dev/null || echo "PR creation failed")

  echo "PR: $PR_URL"

  # Extract PR number and merge
  PR_NUM=$(echo "$PR_URL" | grep -oP '\d+$' || echo "")
  if [ -n "$PR_NUM" ]; then
    log_pr "$PR_NUM" "$PR_URL" "$BRANCH_NAME"
    gh pr merge "$PR_NUM" --repo "$REPO_SLUG" --squash --admin --delete-branch 2>/dev/null && log_merged || echo "Merge failed — PR left open"
  fi

  log_pipeline_end "completed"
  echo ""
  echo "=== Pipeline COMPLETE: $PROJECT_NAME #$ISSUE_NUM ==="
else
  # Tests didn't pass — create draft PR
  PR_URL=$(gh pr create \
    --repo "$REPO_SLUG" \
    --head "$BRANCH_NAME" \
    --title "draft: $ISSUE_TITLE" \
    --body "$(cat <<PREOF
Related: #$ISSUE_NUM

## Pipeline Run — NEEDS MANUAL REVIEW
- Plan: $(basename "$PLAN_FILE")
- Review rounds: $REVIEW_ROUND
- Fix rounds: $FIX_ROUND (max reached)
- Status: Tests still failing after $MAX_FIX_ROUNDS fix rounds

Logs: \`pipeline-logs/$PROJECT_NAME/issue-$ISSUE_NUM/\`
PREOF
)" --draft 2>/dev/null || echo "PR creation failed")

  echo "Draft PR: $PR_URL"
  log_pipeline_end "needs_review"
  echo ""
  echo "=== Pipeline NEEDS REVIEW: $PROJECT_NAME #$ISSUE_NUM ==="
fi

# Return to main
git -C "$PROJECT_DIR" checkout main 2>/dev/null || true
