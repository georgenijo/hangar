#!/usr/bin/env bash
# Pipeline JSON event logging — full lifecycle tracking

PIPELINE_LOGS_ROOT="$HOME/Documents/pipeline-logs"

init_log() {
  local project="$1" issue="$2"
  LOG_DIR="$PIPELINE_LOGS_ROOT/$project/issue-$issue"
  LOG_FILE="$LOG_DIR/pipeline.json"
  mkdir -p "$LOG_DIR"

  if [ ! -f "$LOG_FILE" ]; then
    cat > "$LOG_FILE" <<JSONEOF
{
  "project": "$project",
  "issue": $issue,
  "started_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "ended_at": null,
  "resumed_at": null,
  "status": "running",
  "error": null,
  "complexity": null,
  "model_assignments": null,
  "current_step": null,
  "completed_steps": [],
  "review_rounds": 0,
  "review_approved": false,
  "test_rounds": 0,
  "tests_pass": false,
  "branch": null,
  "pr_number": null,
  "pr_url": null,
  "merged": false,
  "agents": []
}
JSONEOF
  fi
}

log_agent_start() {
  local agent="$1"
  local iteration="${2:-1}"
  local model="${3:-unknown}"
  local tmp=$(mktemp)

  jq --arg agent "$agent" \
     --arg time "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
     --argjson iter "$iteration" \
     --arg model "$model" \
     '.current_step = $agent |
      .agents += [{
        "name": $agent,
        "iteration": $iter,
        "model": $model,
        "started_at": $time,
        "ended_at": null,
        "exit_code": null,
        "duration_sec": null,
        "artifacts": []
      }]' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
}

log_agent_end() {
  local agent="$1" exit_code="$2"
  local end_time duration_sec
  end_time="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  local tmp=$(mktemp)

  # Calculate duration from last agent entry
  local start_time
  start_time=$(jq -r '.agents[-1].started_at // empty' "$LOG_FILE")
  if [ -n "$start_time" ]; then
    local s_epoch e_epoch
    s_epoch=$(date -d "$start_time" +%s 2>/dev/null || echo 0)
    e_epoch=$(date -d "$end_time" +%s 2>/dev/null || echo 0)
    duration_sec=$((e_epoch - s_epoch))
  else
    duration_sec=0
  fi

  jq --arg agent "$agent" \
     --arg time "$end_time" \
     --argjson rc "$exit_code" \
     --argjson dur "$duration_sec" \
     '(.agents[-1]) |= (
        .ended_at = $time |
        .exit_code = $rc |
        .duration_sec = $dur
      ) |
      if $rc == 0 then
        .completed_steps += [$agent]
      else . end' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
}

log_step_artifact() {
  local artifact="$1"
  local tmp=$(mktemp)

  jq --arg artifact "$artifact" \
     '(.agents[-1].artifacts) += [$artifact]' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
}

log_model_assignments() {
  local models_file="$1"
  if [ ! -f "$models_file" ]; then return; fi
  local tmp=$(mktemp)

  jq --slurpfile models "$models_file" \
     '.complexity = $models[0].complexity |
      .model_assignments = $models[0].assignments' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
}

log_review_result() {
  local round="$1" approved="$2"
  local tmp=$(mktemp)

  jq --argjson round "$round" \
     --argjson approved "$approved" \
     '.review_rounds = $round | .review_approved = $approved' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
}

log_test_result() {
  local round="$1" passed="$2"
  local tmp=$(mktemp)

  jq --argjson round "$round" \
     --argjson passed "$passed" \
     '.test_rounds = $round | .tests_pass = $passed' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
}

log_pr() {
  local pr_number="$1" pr_url="$2" branch="$3"
  local tmp=$(mktemp)

  jq --argjson pr "$pr_number" \
     --arg url "$pr_url" \
     --arg branch "$branch" \
     '.pr_number = $pr | .pr_url = $url | .branch = $branch' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
}

log_merged() {
  local tmp=$(mktemp)
  jq '.merged = true' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
}

log_pipeline_end() {
  local status="$1"
  local tmp=$(mktemp)

  jq --arg status "$status" \
     --arg time "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
     '.status = $status | .ended_at = $time' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
}

log_error() {
  local msg="$1"
  local tmp=$(mktemp)

  jq --arg err "$msg" \
     '.error = $err | .status = "failed"' "$LOG_FILE" > "$tmp" && mv "$tmp" "$LOG_FILE"
}
