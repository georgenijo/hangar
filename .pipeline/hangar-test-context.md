# Hangar Test Context (read this before testing any hangar change)

You are testing a change in the **hangar** repo — a Rust+SvelteKit agent control plane that runs on this same box. The dashboard is **already live** and the box is dogfooding itself. Test against the live stack — do not start your own copy.

## Live services (already running, do NOT start your own)

- **Dashboard:** http://localhost:5173 (vite dev, tmux `hangar-frontend`)
- **Backend API:** http://localhost:3000/api/v1 (tmux `hangar-backend`)
- **Supervisor:** systemd user unit `hangar-supervisor.service`, socket at `~/.local/state/hangar/supervisor.sock`
- **DB:** `~/.local/state/hangar/hangar.db`

## Pre-flight (run these first, every test session)

```bash
curl -s localhost:3000/api/v1/health   # expect supervisor_connected: true
tmux ls | grep hangar                  # expect hangar-backend + hangar-frontend
systemctl --user is-active hangar-supervisor   # expect active
```

If any of those fail, halt and report — do not start services yourself, the operator will recover.

## UI testing is MANDATORY for any user-visible change

If your change touches **any** of:
- `frontend/**`
- `backend/src/api/**`
- `backend/src/drivers/**`
- `backend/src/ws/**`
- A backend behavior the dashboard observes (events, output, status, sessions, search)

…then **you MUST drive the dashboard via `agent-browser` and take at least one screenshot per scenario**. API-only testing is insufficient and will be rejected.

For pure backend changes with no UI surface (e.g. log format, internal helper), API testing alone is acceptable, but you must still run the **regression smoke** below to confirm nothing broke in the dashboard.

## agent-browser quick reference

```bash
export PATH=$HOME/.npm-global/bin:$PATH
agent-browser open "http://localhost:5173" --args "--no-sandbox --headless"
agent-browser snapshot                   # accessibility tree (preferred for assertions)
agent-browser screenshot --output /tmp/test-shot-N.png
agent-browser click "button:has-text(\"+ New Session\")"
agent-browser fill "input#slug" "my-test-session"
agent-browser click "button:has-text(\"Create Session\")"
agent-browser close
```

Always `close` at the end of each scenario.

## Regression smoke (run on every test, even pure backend changes)

| # | Scenario | Pass criteria |
|---|---|---|
| 1 | `agent-browser open localhost:5173` then `snapshot` | Topbar shows "+ New Session" button; no JS errors |
| 2 | Click "+ New Session", fill slug `smoke-shell-<rand>`, kind=shell, submit | Modal closes, navigates to session page, terminal pane renders, prompt is colored, `[$HOME]` is `[/home/george]` |
| 3 | Type `ls` + Enter in terminal | Output appears with color codes (verify via screenshot or content snapshot) |
| 4 | Press `Ctrl+\` twice | Sidebar collapses then re-expands |
| 5 | Click red "× Kill" button on the session header | Confirms or removes; session disappears from dashboard list within 2s |

If any smoke step fails AND your change did not intend to alter that surface → mark PASS but flag in `notes`. If your change intended to alter it and it now broke → FAIL.

## Cleanup (mandatory before reporting)

```bash
# DELETE every session your test created
curl -s localhost:3000/api/v1/sessions \
  | python3 -c "import sys,json;[print(s['slug']) for s in json.load(sys.stdin) if s['slug'].startswith('smoke-') or s['slug'].startswith('test-')]" \
  | while read SLUG; do
      curl -s -o /dev/null -w "DELETE %{http_code} $SLUG\n" -X DELETE "localhost:3000/api/v1/sessions/$SLUG"
    done
```

Do NOT delete sessions you did not create.

## Known live bugs — DO NOT refile, only mention if your change affects them

- **#57** SQLite DB malformed (code 267) — DELETE returns 500 on certain pre-existing rows. Surgical sqlite works; integrity_check passes. Suspect FTS5 segments.
- **#58** FTS query 500 on hyphen/colon (`?q=foo-bar` → `no such column: bar`). Plain words OK.
- **#59** Parallel DELETE same session returns 2×204 instead of 204+404.
- **#60** Cost/ctx sidebar lags real CTX line and mislabels "30k" as "output tokens" instead of context.
- **#62** `create_session` has no `cwd` field — caller-specified working dir is ignored (uses hangard cwd).
- **#63** Trailing `event persist failed (code 787) FOREIGN KEY constraint failed` log noise after fast DELETE.

If your change is the fix for one of these, you must include a curl/UI repro showing it now passes.

## Reporting bugs you find that are NOT listed above

- Verify it reproduces twice
- Capture: exact request/response, screenshot, log snippet from `/tmp/hangar-restart-g.log`
- Surface in `notes` of your test results JSON — operator decides whether to file
