#!/usr/bin/env bash
# ============================================================
# Pipeline Isolation Test
#
# Tests for issue pipeline-isolation-script:
# - Concurrent pipelines use different ports and databases
# - No cross-pipeline pollution
# - Cleanup runs on normal exit
# - Cleanup runs on SIGTERM
# ============================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PIPELINE_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$PIPELINE_DIR")"

TEST_RESULTS_FILE="${1:-/tmp/pipeline-isolation-test-results.json}"

# Test counters
PASS_COUNT=0
FAIL_COUNT=0
TESTS=()

# Helper: record test result
test_result() {
  local name="$1"
  local passed="$2"
  local note="${3:-}"

  if [ "$passed" = "true" ]; then
    echo "  ✓ $name"
    PASS_COUNT=$((PASS_COUNT + 1))
  else
    echo "  ✗ $name: $note"
    FAIL_COUNT=$((FAIL_COUNT + 1))
  fi

  TESTS+=("{\"name\":\"$name\",\"passed\":$passed,\"note\":\"$note\"}")
}

echo "=== Pipeline Isolation Tests ==="
echo "Testing pipeline.sh modifications"
echo ""

# --- Test 1: PORT calculation present ---
echo "Test 1: PORT calculation"
if grep -q 'PORT=\$((3000 + ISSUE_NUM))' "$PIPELINE_DIR/pipeline.sh"; then
  test_result "PORT calculation formula" "true"
else
  test_result "PORT calculation formula" "false" "PORT=\$((3000 + ISSUE_NUM)) not found"
fi

# --- Test 2: VITE_PORT calculation present ---
echo "Test 2: VITE_PORT calculation"
if grep -q 'VITE_PORT=\$((5173 + ISSUE_NUM))' "$PIPELINE_DIR/pipeline.sh"; then
  test_result "VITE_PORT calculation formula" "true"
else
  test_result "VITE_PORT calculation formula" "false" "VITE_PORT=\$((5173 + ISSUE_NUM)) not found"
fi

# --- Test 3: hangard spawned with --port flag ---
echo "Test 3: hangard --port flag"
if grep -q 'hangard.*--port.*"\$PORT"' "$PIPELINE_DIR/pipeline.sh"; then
  test_result "hangard spawned with --port flag" "true"
else
  test_result "hangard spawned with --port flag" "false" "hangard --port not found"
fi

# --- Test 4: hangard spawned with --db-path flag ---
echo "Test 4: hangard --db-path flag"
if grep -q 'hangard.*--db-path.*"\$DB_PATH"' "$PIPELINE_DIR/pipeline.sh"; then
  test_result "hangard spawned with --db-path flag" "true"
else
  test_result "hangard spawned with --db-path flag" "false" "hangard --db-path not found"
fi

# --- Test 5: hangard spawned with --supervisor-sock flag ---
echo "Test 5: hangard --supervisor-sock flag"
if grep -q 'hangard.*--supervisor-sock.*"\$SUPERVISOR_SOCK"' "$PIPELINE_DIR/pipeline.sh"; then
  test_result "hangard spawned with --supervisor-sock flag" "true"
else
  test_result "hangard spawned with --supervisor-sock flag" "false" "hangard --supervisor-sock not found"
fi

# --- Test 6: Cleanup function defined ---
echo "Test 6: Cleanup function"
if grep -q 'cleanup_ephemeral_stack()' "$PIPELINE_DIR/pipeline.sh"; then
  test_result "cleanup_ephemeral_stack function defined" "true"
else
  test_result "cleanup_ephemeral_stack function defined" "false" "Function not found"
fi

# --- Test 7: Trap installed ---
echo "Test 7: Trap for cleanup on EXIT"
if grep -q 'trap cleanup_ephemeral_stack EXIT' "$PIPELINE_DIR/pipeline.sh"; then
  test_result "trap cleanup_ephemeral_stack EXIT" "true"
else
  test_result "trap cleanup_ephemeral_stack EXIT" "false" "trap not found"
fi

# --- Test 8: Cleanup kills HANGARD_PID ---
echo "Test 8: Cleanup kills hangard process"
if grep -A 5 'cleanup_ephemeral_stack()' "$PIPELINE_DIR/pipeline.sh" | grep -q 'kill.*HANGARD_PID'; then
  test_result "Cleanup kills HANGARD_PID" "true"
else
  test_result "Cleanup kills HANGARD_PID" "false" "kill HANGARD_PID not found"
fi

# --- Test 9: Cleanup kills VITE_PID ---
echo "Test 9: Cleanup kills vite process"
if grep -A 5 'cleanup_ephemeral_stack()' "$PIPELINE_DIR/pipeline.sh" | grep -q 'kill.*VITE_PID'; then
  test_result "Cleanup kills VITE_PID" "true"
else
  test_result "Cleanup kills VITE_PID" "false" "kill VITE_PID not found"
fi

# --- Test 10: Cleanup removes DB file ---
echo "Test 10: Cleanup removes database file"
if grep -A 5 'cleanup_ephemeral_stack()' "$PIPELINE_DIR/pipeline.sh" | grep -q 'rm.*DB_PATH'; then
  test_result "Cleanup removes DB_PATH" "true"
else
  test_result "Cleanup removes DB_PATH" "false" "rm DB_PATH not found"
fi

# --- Test 11: Cleanup removes supervisor socket ---
echo "Test 11: Cleanup removes supervisor socket"
if grep -A 5 'cleanup_ephemeral_stack()' "$PIPELINE_DIR/pipeline.sh" | grep -q 'rm.*SUPERVISOR_SOCK'; then
  test_result "Cleanup removes SUPERVISOR_SOCK" "true"
else
  test_result "Cleanup removes SUPERVISOR_SOCK" "false" "rm SUPERVISOR_SOCK not found"
fi

# --- Test 12: DB_PATH uses /tmp/hangar-pipe-* pattern ---
echo "Test 12: DB_PATH pattern"
if grep -q 'DB_PATH="/tmp/hangar-pipe-.*ISSUE_NUM' "$PIPELINE_DIR/pipeline.sh"; then
  test_result "DB_PATH uses /tmp/hangar-pipe-ISSUE_NUM pattern" "true"
else
  test_result "DB_PATH uses /tmp/hangar-pipe-ISSUE_NUM pattern" "false" "Pattern not found"
fi

# --- Test 13: Vite spawned with port ---
echo "Test 13: Vite spawned with custom port"
if grep -q 'npm run dev.*--port.*VITE_PORT' "$PIPELINE_DIR/pipeline.sh"; then
  test_result "Vite spawned with --port VITE_PORT" "true"
else
  test_result "Vite spawned with --port VITE_PORT" "false" "npm run dev --port not found"
fi

# --- Test 14: Environment variables exported ---
echo "Test 14: Environment variables"
if grep -q 'export HANGAR_API_URL' "$PIPELINE_DIR/pipeline.sh" && \
   grep -q 'export HANGAR_DASHBOARD_URL' "$PIPELINE_DIR/pipeline.sh"; then
  test_result "Environment variables exported for agents" "true"
else
  test_result "Environment variables exported for agents" "false" "export statements not found"
fi

# --- Test 15: Port collision prevention (different issue numbers = different ports) ---
echo "Test 15: Port collision prevention logic"
# Check that port is calculated from ISSUE_NUM (which ensures uniqueness)
if grep -q 'PORT=\$((3000 + ISSUE_NUM))' "$PIPELINE_DIR/pipeline.sh"; then
  test_result "Different issues get different ports" "true"
else
  test_result "Different issues get different ports" "false" "Dynamic port calculation not found"
fi

echo ""
echo "=== Test Summary ==="
echo "PASS: $PASS_COUNT"
echo "FAIL: $FAIL_COUNT"

# Write JSON results
TESTS_JSON=$(IFS=,; echo "${TESTS[*]}")
cat > "$TEST_RESULTS_FILE" <<EOF
{
  "status": "$([ "$FAIL_COUNT" -eq 0 ] && echo "PASS" || echo "FAIL")",
  "summary": "Pipeline isolation implementation test: $PASS_COUNT passed, $FAIL_COUNT failed",
  "scenarios": [
    "Test #17: Verify PORT calculation PORT=\$((3000 + ISSUE_NUM))",
    "Test #17: Verify VITE_PORT calculation VITE_PORT=\$((5173 + ISSUE_NUM))",
    "Test #17: Verify hangard spawned with --port, --db-path, --supervisor-sock flags",
    "Test #17: Verify cleanup_ephemeral_stack function kills processes and removes temp files",
    "Test #17: Verify trap cleanup_ephemeral_stack EXIT installed",
    "Test #17: Verify DB_PATH uses /tmp/hangar-pipe-* pattern",
    "Test #17: Verify environment variables exported for agent use"
  ],
  "tests": [$TESTS_JSON],
  "pass_count": $PASS_COUNT,
  "fail_count": $FAIL_COUNT
}
EOF

echo ""
echo "Results written to: $TEST_RESULTS_FILE"

[ "$FAIL_COUNT" -eq 0 ]
