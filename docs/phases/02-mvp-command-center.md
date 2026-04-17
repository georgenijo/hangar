# Phase 2 — MVP Command Center

**Status:** ⬜ Planned (largest phase)

## Goal

Replace the Phase 0 ttyd-based dashboard with a real product. Own the PTYs. Parse Claude Code output into a chat-like UI. Expose a REST API for prompting from anywhere. Push to George's phone when a session is waiting on him. Deliver a dashboard that feels like a command center — visibility of every running agent at a glance — because that's the killer feature.

This phase bundles what would naturally be three smaller phases (backend, rich UI, push + API) into one ship. The rationale: none of the three are individually worth shipping without the others, and the dashboard-as-killer-feature demand requires the bundle.

## Non-goals

- No sandboxing (containers, overlayfs, network policy) — later phase
- No recording / replay / cross-session search — Phase 5
- No Codex rich driver — Phase 4 (Codex still runs via RawBytes driver meanwhile)
- No branching / forking of sessions — deferred
- No multi-node — single box
- No inter-agent protocols — Phase 9
- No voice UI — Phase 11

## Deliverables

### Backend (`backend/`)

Rust workspace (`cargo` single crate, split into modules):

- `hangard` — binary with `tokio` runtime, `axum` HTTP server, `tower-http` middleware
- `session` module — `Session`, `SessionKind`, `SessionState`, state machine, SQLite persistence
- `pty` module — wraps `portable-pty`, owns child process lifecycle, exposes byte streams
- `supervisor` — holds PTY fds across backend restart via Unix socket handoff (or falls back to systemd re-parenting; spike)
- `ringbuf` module — 100 MB per-session output ring file with offset/length API
- `events` module — `Event` enum, persistent log, broadcast bus (`tokio::sync::broadcast`)
- `drivers/shell.rs`, `drivers/claude_code.rs`, `drivers/raw_bytes.rs`
- `api/` — `/api/v1/*` HTTP handlers
- `ws/` — `/ws/v1/*` WebSocket handlers
- `push.rs` — rule engine + ntfy adapter (APNs optional stretch)
- `cc_hook_socket.rs` — `localhost:3000/_cc_hook` receiver for Claude Code hook payloads
- `metrics.rs` — `/api/v1/metrics`
- `config.rs` — `~/.config/hangar/config.toml` loader

### Frontend (`web/`)

Replace Phase 0 plain HTML with SvelteKit app:

- Landing: command-center view — tiles for every session, live badges (state, tokens, idle time, model)
- Session detail: chat-style view for Claude Code (turn bubbles, collapsible thinking, tool-call cards), plain terminal for Shell and RawBytes (xterm.js)
- Global search bar (future — hook up to Phase 5)
- Global spawn button → modal with kind/cwd/slug form
- Sidebar: filters by label
- Theme: dark-mode-first, monospace code blocks, compact density
- WebSocket reconnect with exponential backoff

### Ops

- `systemd/hangar.service` — binary supervision, restart on-failure
- `caddy/Caddyfile` updated — reverse-proxy `/api` + `/ws` to backend, still serves the new Svelte build for everything else
- Old `/s/<name>` iframe routes retained as a compatibility view while migrating
- `scripts/deploy.sh` learns a `backend` subcommand: `git pull; cargo build --release; systemctl restart hangar`
- `.github/workflows/ci.yml` — `cargo test`, `cargo clippy`, `svelte-check` on every push
- `CHANGELOG.md` scaffolded

### Docs

- Phase 2 ships its own migration notes in `docs/phases/02-mvp-command-center.md#migration`
- SESSION-PROTOCOL.md revised if spec drift

## Acceptance criteria

- `systemctl status hangar` is active after reboot
- All existing tmux sessions migrate to hangar-owned PTYs (tmux-only holdouts are explicitly documented)
- Dashboard lists every session without user config — spawning a new one via API makes it appear
- Claude Code sessions render as chat (turn bubbles, not raw terminal) and show `state`, `model`, `tokens_used`, `awaiting` badges updated in real time via WebSocket
- Shell and RawBytes sessions render as xterm.js terminal, keyboard input works, resize works
- `POST /api/v1/sessions/:id/prompt` sends text to a Claude session and the turn appears in the dashboard within 1 s
- `POST /api/v1/broadcast` reaches N sessions
- When any Claude session transitions to `Awaiting`, an ntfy push hits George's phone within 5 s with a deep link back to that session
- Backend crash → `systemctl restart hangar` → all sessions still alive, dashboard reconnects
- 100 MB ring buffer rotates correctly (tested by deliberately flooding output)
- `/api/v1/metrics` returns JSON with `sessions_active`, `tokens_today`, `rss_mb`, `uptime_s`
- Page load (cold) < 2 s on LAN; reattach to existing terminal < 500 ms
- CI passes on every push to `main`
- `docs/phases/02-*.md` marked ✅ and a CHANGELOG entry is written

## Dependencies

- Phase 0 (running) and Phase 1 (for phone access to push deep links)
- Rust toolchain on box (`rustup`, `cargo`)
- `npm` + `pnpm` for frontend
- `ntfy` chosen as push (self-hosted on box) OR Pushover account
- Claude Code version supports hooks we need (`SessionStart`, `UserPromptSubmit`, `Stop`, `PreToolUse`, `PostToolUse`, `Notification`)

## Risks / unknowns

- **PTY supervisor across backend restart** — hardest part. Two paths:
  - (a) separate tiny supervisor process owns the PTY fd, hands it back via Unix socket on reconnect
  - (b) systemd `Type=notify` + socket activation: backend crashes, systemd restarts, fds remain attached
  - Spike first to pick one; ADR required
- **Claude Code hook stability** — hooks are a moving target in CC releases; parser fallback must be robust
- **SvelteKit build size + SSR mode** — decide SPA vs SSR; SPA simpler for embedded serving via Caddy
- **xterm.js + Claude Code ANSI quirks** — CC uses non-trivial cursor movements and alternate-screen; test early
- **ntfy self-host vs hosted ntfy.sh** — self-host on box adds ops; hosted ntfy.sh requires topic secrecy. ADR.
- **Token counting accuracy** — hook-provided counts are authoritative; parser estimates otherwise. Flag "estimated" in UI.

## Estimated effort

- Claude-sessions of work: large. Likely 6–10 sessions spread over weeks.
- Suggested milestones inside this phase:
  1. Backend skeleton + shell driver + xterm.js terminal (first vertical slice)
  2. SQLite + ring file + event persistence
  3. Supervisor / crash recovery
  4. Claude Code driver (hooks + parser)
  5. Svelte chat UI for Claude sessions
  6. REST API surface complete
  7. Push rules + ntfy integration
  8. Polish, metrics, CI

Each milestone should be a PR with its own tests.

## Migration

1. Phase 0 continues to serve at `http://optiplex:8080` on `/s/<name>` during the whole Phase 2 build
2. Once backend is running in parallel on `localhost:3000`, Caddy routes `/api` + `/ws` to it and serves the new Svelte build at `/`
3. After all acceptance criteria met and Phase 0 routes unused for a week, remove the ttyd systemd units and the `/s/<name>` Caddy block
4. Commit removal in a separate PR with CHANGELOG entry

## Rollback

- Revert `hangar.service` to `systemctl stop hangar` + disable
- Restore Phase 0 Caddy routes (kept in git history as `Caddyfile.phase0`)
- ttyd units remain enabled throughout Phase 2 build, so rollback is instant

## Out of scope for this phase (issue-tracked)

- Auto-update from GitHub releases
- Per-user API tokens
- Grafana-grade metrics
- Replay UI
- Branching
- Sandboxing
- Multi-node registration
