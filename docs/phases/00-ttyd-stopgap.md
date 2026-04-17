# Phase 0 — ttyd stopgap dashboard

**Status:** ✅ Shipped

## Goal

Put a useful dashboard in front of the tmux sessions already running on the box, without writing any backend code. Buy time for the Rust MVP by making the existing tmux workflow immediately visible and shareable over the tailnet.

## Non-goals

- No auth (tailnet is the boundary)
- No public URL (that's Phase 1)
- No session spawning from the UI (still `tmux new-session`)
- No intelligence, parsing, or push

## Deliverables

- `caddy/Caddyfile` — reverse-proxy `/s/<name>` → ttyd ports, static serve dashboard
- `systemd/ttyd-{codex,wave,issue12}.service` — one ttyd per tmux session
- `web/index.html` — dashboard: tabs, grid view, per-tile reload, global reload, link out to Cockpit
- `scripts/deploy.sh` — idempotent install script
- `README.md`

## Acceptance criteria

- `http://optiplex:8080` loads from any tailnet device ✅
- Dashboard shows one tab per tmux session ✅
- Grid view shows all three sessions side-by-side ✅
- Per-tab and global reload buttons refresh iframes without losing focus ✅
- All three ttyd services are managed by systemd and restart on failure ✅
- Writing to any iframe works (not read-only) ✅
- Caddy survives reboot (enabled) ✅

## Dependencies

- `ttyd`, `caddy` installed on the box
- Existing tmux session `work` with windows `codex`, `wave`, `issue12`
- Linked ungrouped sessions named `codex`, `wave`, `issue12` (so per-pane attach doesn't share active-window state)

## Rollout

`./scripts/deploy.sh` on the box. Already executed, commit `e5ec0de`.

## Rollback

`systemctl stop ttyd-*` + `systemctl stop caddy`. The underlying tmux sessions are untouched and `ssh` + `tmux attach` works as always.

## Known limitations (acknowledged, kept for now)

- Iframes occasionally hang; reload button is the current workaround — addressed properly by Phase 2's own WebSocket reconnect
- Hardcoded session list in HTML; adding a new session requires editing `web/index.html`, `Caddyfile`, and creating a systemd unit — Phase 2 auto-discovers
- No auth; anyone on the tailnet can type into any session — explicit trust model
