#!/usr/bin/env bash
# hangar regression smoke — runs agent-browser through the dashboard
# golden path. Exits non-zero on any failure so the pipeline can treat
# it as a test failure and route to fixer.
#
# Usage: ./.pipeline/smoke.sh [output-dir]
#   output-dir defaults to /tmp/hangar-smoke-$(date +%s)
set -uo pipefail

OUT_DIR="${1:-/tmp/hangar-smoke-$(date +%s)}"
mkdir -p "$OUT_DIR"

DASHBOARD_URL="${HANGAR_DASHBOARD_URL:-http://localhost:5173}"
API_URL="${HANGAR_API_URL:-http://localhost:3000/api/v1}"
SLUG="smoke-pipeline-$(date +%s)-$RANDOM"

export PATH="$HOME/.npm-global/bin:$PATH"

fail() {
  echo "[smoke] FAIL: $*" >&2
  agent-browser close >/dev/null 2>&1 || true
  curl -s -o /dev/null -X DELETE "$API_URL/sessions/$SLUG" || true
  exit 1
}

step() { echo "[smoke] $*"; }

# Pre-flight — JSON parse via python to avoid quote hell
step "pre-flight: api health"
H=$(curl -s --max-time 3 "$API_URL/health" || echo "")
[ -n "$H" ] || fail "health endpoint unreachable"
python3 -c "
import sys, json
try:
    d = json.loads(sys.argv[1])
except Exception as e:
    sys.exit(f'health response not JSON: {e}')
if d.get('status') != 'ok':
    sys.exit(f'status != ok: {d}')
if not d.get('supervisor_connected'):
    sys.exit(f'supervisor not connected: {d}')
" "$H" || fail "health JSON failed: $H"

# Open dashboard
step "open dashboard"
agent-browser open "$DASHBOARD_URL" --args "--no-sandbox --headless" \
  > "$OUT_DIR/01-open.txt" 2>&1 || fail "agent-browser open failed (see $OUT_DIR/01-open.txt)"

# Wait for SvelteKit hydration — vite dev server can take a sec on first paint
step "wait for dashboard to render"
for i in 1 2 3 4 5 6 7 8; do
  agent-browser snapshot > "$OUT_DIR/02-dashboard.snap" 2>&1 || true
  if grep -q "Hangar" "$OUT_DIR/02-dashboard.snap" 2>/dev/null; then
    break
  fi
  sleep 1
done
grep -q "Hangar" "$OUT_DIR/02-dashboard.snap" || fail "Hangar header not in dashboard snapshot"
grep -q "New Session" "$OUT_DIR/02-dashboard.snap" || fail "New Session button missing"
agent-browser screenshot --output "$OUT_DIR/02-dashboard.png" >/dev/null 2>&1 || true

# Spawn session via API (bypass modal — modal can be in deeper tester suites later)
step "spawn session $SLUG via API"
SPAWN_RESP=$(curl -s -X POST "$API_URL/sessions" \
  -H "Content-Type: application/json" \
  -d "{\"slug\":\"$SLUG\",\"kind\":{\"type\":\"shell\"}}")
SESSION_ID=$(python3 -c "
import sys, json
try:
    d = json.loads(sys.argv[1])
    print(d.get('id', ''))
except Exception:
    pass
" "$SPAWN_RESP")
[ -n "$SESSION_ID" ] || fail "spawn API returned: $SPAWN_RESP"
sleep 1

# Visit session page
step "navigate to session page"
agent-browser open "$DASHBOARD_URL/session/$SESSION_ID" --args "--no-sandbox --headless" \
  > "$OUT_DIR/03-session-open.txt" 2>&1 || fail "session open failed"
for i in 1 2 3 4 5 6 7 8; do
  agent-browser snapshot > "$OUT_DIR/04-session.snap" 2>&1 || true
  if grep -q "$SLUG" "$OUT_DIR/04-session.snap" 2>/dev/null; then
    break
  fi
  sleep 1
done
grep -q "$SLUG" "$OUT_DIR/04-session.snap" || fail "session slug $SLUG not visible on session page"
agent-browser screenshot --output "$OUT_DIR/04-session.png" >/dev/null 2>&1 || true

# Verify env-fix shipped: shell prompt should show /home/george, not empty path
step "verify env fix (HOME populated in PTY)"
sleep 2
PROBE_FILE="/tmp/smoke-probe-$SLUG.out"
rm -f "$PROBE_FILE"
PROMPT_HTTP=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST "$API_URL/sessions/$SLUG/prompt" \
  -H "Content-Type: application/json" \
  -d "{\"text\":\"echo ENV_HOME=[\$HOME] ENV_TERM=[\$TERM] > $PROBE_FILE\"}")
[ "$PROMPT_HTTP" = "204" ] || fail "prompt POST returned $PROMPT_HTTP"
sleep 2
PTY_OUT=$(cat "$PROBE_FILE" 2>/dev/null || echo "")
echo "$PTY_OUT" | grep -q "ENV_HOME=\[/home/george\]" || fail "HOME not populated in PTY: '$PTY_OUT'"
echo "$PTY_OUT" | grep -q "ENV_TERM=\[xterm" || fail "TERM not xterm-*: '$PTY_OUT'"

# Cleanup: DELETE the smoke session
step "cleanup: DELETE $SLUG"
DEL_HTTP=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$API_URL/sessions/$SLUG")
[ "$DEL_HTTP" = "204" ] || fail "DELETE returned $DEL_HTTP (expected 204)"

# Cleanup any other smoke-pipeline-* sessions
curl -s "$API_URL/sessions" | python3 -c "
import sys, json
for s in json.load(sys.stdin):
  if s['slug'].startswith('smoke-pipeline-'):
    print(s['slug'])
" | while read STALE; do
  curl -s -o /dev/null -X DELETE "$API_URL/sessions/$STALE" || true
done

agent-browser close >/dev/null 2>&1 || true
rm -f "$PROBE_FILE"

step "smoke PASS — artifacts in $OUT_DIR"
exit 0
