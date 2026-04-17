#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# Batch Pipeline Runner
# Usage: batch.sh [--project-dir /path] [--issues 15,16,17] [--all-open]
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PROJECT_DIR="$(pwd)"
ISSUES=""
ALL_OPEN=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --project-dir) PROJECT_DIR="$2"; shift 2 ;;
    --issues) ISSUES="$2"; shift 2 ;;
    --all-open) ALL_OPEN=true; shift ;;
    *) echo "Unknown arg: $1"; exit 1 ;;
  esac
done

REPO_SLUG=$(git -C "$PROJECT_DIR" remote get-url origin | sed 's|.*github.com[:/]||;s|\.git$||')

# Get issue list
if [ "$ALL_OPEN" = true ]; then
  ISSUE_LIST=$(gh issue list --repo "$REPO_SLUG" --state open --json number -q '.[].number' | sort -n)
elif [ -n "$ISSUES" ]; then
  ISSUE_LIST=$(echo "$ISSUES" | tr ',' '\n')
else
  echo "Usage: batch.sh [--project-dir /path] [--issues 15,16,17 | --all-open]"
  exit 1
fi

echo "=== Batch Pipeline ==="
echo "Project: $PROJECT_DIR"
echo "Issues:  $(echo $ISSUE_LIST | tr '\n' ' ')"
echo ""

TOTAL=0
PASSED=0
FAILED=0

for issue in $ISSUE_LIST; do
  TOTAL=$((TOTAL + 1))
  echo ""
  echo "################################################################"
  echo "# Issue #$issue ($TOTAL of $(echo "$ISSUE_LIST" | wc -l | tr -d ' '))"
  echo "################################################################"

  if "$SCRIPT_DIR/pipeline.sh" "$issue" --project-dir "$PROJECT_DIR"; then
    PASSED=$((PASSED + 1))
  else
    FAILED=$((FAILED + 1))
    echo "⚠ Issue #$issue failed — continuing to next"
  fi

  # Return to main between issues
  git -C "$PROJECT_DIR" checkout main 2>/dev/null || true
done

echo ""
echo "=== Batch Complete ==="
echo "Total: $TOTAL | Passed: $PASSED | Failed: $FAILED"
