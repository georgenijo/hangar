#!/usr/bin/env bash
set -euo pipefail

# Test script for tester gate validation
# Tests AC13, AC14, AC15 from issue-eab8e24c-06-pipeline-tester-gate

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TESTS_DIR="$SCRIPT_DIR"
ISSUE_NUM=42  # Test with issue number 42

echo "=== Testing Tester Gate Validation ==="
echo ""

# Helper function to simulate gate checks
run_gate_checks() {
  local test_results="$1"
  local gate_pass=true
  local gate_notes=""

  if [ -f "$test_results" ]; then
    # Check scenarios.length >= 1
    SCENARIO_COUNT=$(jq '(.scenarios // []) | length' "$test_results" 2>/dev/null || echo 0)
    if [ "$SCENARIO_COUNT" -lt 1 ]; then
      echo "    Gate FAIL: scenarios.length >= 1 required, got $SCENARIO_COUNT"
      gate_pass=false
      gate_notes="${gate_notes}scenarios.length >= 1 required (got $SCENARIO_COUNT); "
    fi

    # Check at least one scenario references issue ID
    ISSUE_REF=$(jq -e ".scenarios[]? | select(. | test(\"#${ISSUE_NUM}|issue-${ISSUE_NUM}\"))" "$test_results" 2>/dev/null || echo "")
    if [ -z "$ISSUE_REF" ] && [ "$SCENARIO_COUNT" -gt 0 ]; then
      echo "    Gate FAIL: at least one scenario must reference issue #$ISSUE_NUM or issue-$ISSUE_NUM"
      gate_pass=false
      gate_notes="${gate_notes}scenario must reference issue ID; "
    fi

    # Check summary non-empty
    SUMMARY_LEN=$(jq '(.summary // "") | length' "$test_results" 2>/dev/null || echo 0)
    if [ "$SUMMARY_LEN" -lt 1 ]; then
      echo "    Gate FAIL: summary must be non-empty"
      gate_pass=false
      gate_notes="${gate_notes}summary must be non-empty; "
    fi
  fi

  if [ "$gate_pass" = true ]; then
    echo "    ✓ Gate PASS"
    return 0
  else
    echo "    ✗ Gate FAIL: $gate_notes"
    return 1
  fi
}

# Test 1: Empty scenarios array should fail
echo "Test 1: Empty scenarios array"
if run_gate_checks "$TESTS_DIR/test-results-empty-scenarios.json"; then
  echo "❌ FAILED: Should have rejected empty scenarios"
  exit 1
else
  echo "✅ PASSED: Correctly rejected empty scenarios"
fi
echo ""

# Test 2: Scenario without issue ID should fail
echo "Test 2: Scenario without issue ID reference"
if run_gate_checks "$TESTS_DIR/test-results-missing-issue-ref.json"; then
  echo "❌ FAILED: Should have rejected scenarios without issue ID"
  exit 1
else
  echo "✅ PASSED: Correctly rejected scenarios without issue ID"
fi
echo ""

# Test 3: Empty summary should fail
echo "Test 3: Empty summary field"
if run_gate_checks "$TESTS_DIR/test-results-empty-summary.json"; then
  echo "❌ FAILED: Should have rejected empty summary"
  exit 1
else
  echo "✅ PASSED: Correctly rejected empty summary"
fi
echo ""

# Test 4: Valid data should pass
echo "Test 4: Valid test results"
if run_gate_checks "$TESTS_DIR/test-results-valid.json"; then
  echo "✅ PASSED: Correctly accepted valid test results"
else
  echo "❌ FAILED: Should have accepted valid test results"
  exit 1
fi
echo ""

echo "=== All Tests PASSED ==="
