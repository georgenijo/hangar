#!/usr/bin/env bash
# Crash recovery — read checkpoint, determine where to resume

get_last_completed_step() {
  local log_file="$1"
  if [ ! -f "$log_file" ]; then
    echo ""
    return
  fi
  jq -r '.completed_steps[-1] // ""' "$log_file"
}

get_pipeline_status() {
  local log_file="$1"
  if [ ! -f "$log_file" ]; then
    echo "new"
    return
  fi
  jq -r '.status // "unknown"' "$log_file"
}

should_skip_step() {
  local log_file="$1" step="$2"
  if [ ! -f "$log_file" ]; then
    return 1  # don't skip
  fi
  local completed
  completed=$(jq -r --arg step "$step" '.completed_steps | index($step) != null' "$log_file")
  [ "$completed" = "true" ]
}

get_review_iteration() {
  local log_file="$1"
  if [ ! -f "$log_file" ]; then
    echo "0"
    return
  fi
  jq '[.agents[] | select(.name == "senior-reviewer")] | length' "$log_file"
}

get_fix_iteration() {
  local log_file="$1"
  if [ ! -f "$log_file" ]; then
    echo "0"
    return
  fi
  jq '[.agents[] | select(.name == "fixer")] | length' "$log_file"
}

mark_resumed() {
  local log_file="$1"
  local tmp=$(mktemp)
  jq --arg time "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
     '.status = "running" | .resumed_at = $time' "$log_file" > "$tmp" && mv "$tmp" "$log_file"
}
