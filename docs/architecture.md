# Hangar Architecture

This document captures the current wiring of Hangar (backend + frontend + supervisor daemon + Claude hook loop + sandboxing). Kept at a high level; for details, see the phase docs in `docs/phases/` and the ADRs in `docs/decisions/`.

## System overview

```
                                      ┌──────────────┐
                                      │   User       │
                                      │  (browser    │
                                      │   or phone)  │
                                      └──────┬───────┘
                                             │ HTTPS
                                             ▼
                                    ┌─────────────────┐
                                    │   Cloudflare    │   ← Phase 1
                                    │  Tunnel+Access  │
                                    └────────┬────────┘
                                             │
                                             ▼
                    ┌────────────────────────────────────────────┐
                    │     Hangar frontend (SvelteKit @ :5173)    │
                    │  ┌──────────┐  ┌─────────────┐  ┌───────┐  │
                    │  │Dashboard │  │ SessionView │  │ /logs │  │
                    │  └──────────┘  │ (xterm.js + │  └───────┘  │
                    │                │ InsightsPanel)            │
                    │                └─────────────┘             │
                    └──────────────┬─────────────────────────────┘
                                   │ REST + WebSocket
                                   ▼
    ┌───────────────────────────────────────────────────────────────────┐
    │                   hangard (Rust backend @ :3000)                  │
    │                                                                   │
    │   ┌──────────────┐      ┌─────────────────────────────┐           │
    │   │  REST API    │      │        Event Bus            │           │
    │   │ /sessions    │◄────►│   tokio broadcast channel   │           │
    │   │ /prompt /key │      │   TurnStarted, CostUpdated, │           │
    │   │ /output      │      │   ToolCall*, ModelChanged…  │           │
    │   │ /broadcast   │      └────────────┬────────────────┘           │
    │   │ /search FTS5 │                   │                            │
    │   │ /metrics     │                   │ fan-out                    │
    │   └──────────────┘                   │                            │
    │                                      ▼                            │
    │   ┌──────────────┐      ┌─────────────────────────────┐           │
    │   │  WebSocket   │◄─────│  Per-session handlers       │           │
    │   │ /ws/pty      │      │  ┌─────────────────────┐    │           │
    │   │ /ws/logs     │      │  │ Driver              │    │           │
    │   └──────────────┘      │  │  claude_code /      │    │           │
    │                         │  │  codex / shell /    │    │           │
    │                         │  │  raw_bytes          │    │           │
    │                         │  └─────────┬───────────┘    │           │
    │                         │            │                │           │
    │                         │            ▼                │           │
    │                         │  ┌─────────────────────┐    │           │
    │                         │  │ status_scraper      │    │           │
    │                         │  │ (regex on bytes:    │    │           │
    │                         │  │ CTX% / tokens / $ / │    │           │
    │                         │  │ model / tool calls) │    │           │
    │                         │  └─────────────────────┘    │           │
    │                         │                             │           │
    │                         │  ┌─────────────────────┐    │           │
    │                         │  │ ring buffer         │    │           │
    │                         │  │ (100MB transcript   │    │           │
    │                         │  │ per session on disk)│    │           │
    │                         │  └─────────────────────┘    │           │
    │                         └─────────────────────────────┘           │
    │                                      ▲                            │
    │                                      │ spawn                      │
    │   ┌──────────────────────────────────┴───────────┐                │
    │   │            PTY spawn path (one of):          │                │
    │   │ spawn_pty / spawn_pty_supervised /           │                │
    │   │ spawn_pty_sandboxed (Phase 6 podman)         │                │
    │   └──────────────────────┬───────────────────────┘                │
    │                          │                                        │
    │   ┌──────────────────────┴────────────────────────┐               │
    │   │                                               │               │
    │   ▼                                               ▼               │
    │ ┌─────────┐                                ┌────────────────┐     │
    │ │ SQLite  │                                │/_cc_hook       │     │
    │ │ sessions│                                │  (POST from    │     │
    │ │ events  │◄───persist events──────────────│  Claude hooks) │     │
    │ │ FTS5    │                                └────────────────┘     │
    │ └─────────┘                                                       │
    │       │                                                           │
    │       ▼                                                           │
    │ ┌─────────┐                                                       │
    │ │  push   │──► ntfy (Phase 2.7)                                   │
    │ │  rules  │                                                       │
    │ └─────────┘                                                       │
    └────────┬──────────────────────────────────────────────────────────┘
             │ SCM_RIGHTS fd passing over Unix socket
             ▼
    ┌────────────────────────────────────┐         ┌───────────────────┐
    │ hangar-supervisor (separate binary)│◄───────►│   Real PTY +      │
    │   Holds PTY fds                    │  owns   │  child process    │
    │   Survives hangard restart         │         │  (claude, codex,  │
    │   sock: ~/.local/state/hangar/     │         │   bash…)          │
    │         supervisor.sock            │         └───────────────────┘
    └────────────────────────────────────┘                 ▲
                                                           │ bytes
                                                           │
                                            ┌──────────────┴─────────┐
                                            │  Podman sandbox        │
                                            │  (Phase 6, optional    │
                                            │  per-session)          │
                                            └────────────────────────┘
```

## Critical flows

### User prompts a session
1. Browser opens WebSocket to `/ws/v1/sessions/:id/pty`
2. User types, frontend sends `POST /api/v1/sessions/:id/prompt {"text": "..."}`
3. Backend's driver writes prompt bytes to the PTY master
4. Claude (or whichever agent) processes the prompt

### Output streams back
1. PTY master emits bytes via `tokio::io` → driver's `on_bytes()`
2. `status_scraper` regex-scans for live metrics (CTX%, tokens, cost, model, tool calls) and emits structured `AgentEvent`s
3. Bytes written to the session's ring buffer file (rolling 100MB per session)
4. Events broadcast to the event bus (tokio broadcast channel)
5. WebSocket handler drains the bus and pushes updates to the browser (terminal bytes + InsightsPanel events)

### Claude hooks enrich the stream
1. Claude Code is spawned with a generated `settings.json` pointing hooks at `http://localhost:3000/_cc_hook`
2. On Claude lifecycle transitions (`Start`, `Stop`, `PreToolUse`, `PostToolUse`, `UserPromptSubmit`, `Notification`), Claude fires an HTTPS POST with structured JSON
3. `cc_hook_socket` receives the payload, maps to the session, emits events onto the bus
4. These events are authoritative (vs the regex scraper which is a best-effort fallback)

### Session survival across backend restarts
1. Session spawn goes through `spawn_pty_supervised` when the supervisor daemon is reachable
2. Supervisor spawns the PTY + child, sends the master fd to hangard over Unix socket via `SCM_RIGHTS`
3. Hangard holds a fd reference, reads bytes
4. On hangard restart: the child and PTY are owned by the supervisor — they keep running
5. New hangard connects to supervisor, lists live sessions, re-attaches byte streams via `attach_fd`
6. Session state in DB reconciles with supervisor's live list (DB-only rows marked `exited`)

### Push notifications
1. Event fires on the bus
2. Push-rules engine evaluates (e.g., "state changed to `waiting`" + "user opted in for session X")
3. Matching rules fire to ntfy (self-hosted)
4. User gets phone/desktop notification

## Components by directory

| Path | Role |
|---|---|
| `backend/src/main.rs` | Binary entry; wires AppState, starts axum server, discovers/connects supervisor |
| `backend/src/api/` | REST handlers (sessions, prompt, key, output, events, search, broadcast, metrics, logs) |
| `backend/src/ws/` | WebSocket handlers (pty, logs) |
| `backend/src/drivers/mod.rs` | `AgentDriver` trait, `DriverRegistry`, shared `status_scraper` module |
| `backend/src/drivers/claude_code.rs` | Claude Code driver — hook-integrated, turn tracking, structured events |
| `backend/src/drivers/codex.rs`, `shell.rs`, `raw_bytes.rs` | Other agent drivers |
| `backend/src/pty.rs` | PTY spawn paths (`spawn_pty`, `spawn_pty_supervised`, `spawn_pty_sandboxed`) |
| `backend/src/supervisor_client.rs`, `supervisor_protocol.rs` | Client for `hangar-supervisor` daemon, protocol types |
| `backend/src/bin/hangar-supervisor.rs` | Separate supervisor daemon binary |
| `backend/src/ringbuf.rs` | Rolling per-session transcript on disk |
| `backend/src/events.rs` | `AgentEvent` enum + persistence + search indexing |
| `backend/src/cc_hook_socket.rs` | Claude Code hook receiver (`/_cc_hook`) |
| `backend/src/push.rs` | Rule engine + ntfy adapter |
| `backend/src/sandbox/` | Podman + overlayfs + nftables orchestration (Phase 6) |
| `frontend/src/routes/` | SvelteKit pages (dashboard, session, logs) |
| `frontend/src/lib/components/TerminalView.svelte` | xterm.js wrapper |
| `frontend/src/lib/components/InsightsPanel.svelte` | Right-side structured event panel |
| `frontend/src/lib/stores/events.svelte.ts` | Event store (WebSocket subscription, derived metrics) |
| `systemd/hangar.service`, `hangar-supervisor.service` | Production deployment units |

## Deployment shape

Hangar is designed to run on one personal Linux box exposed over Tailscale (or via Cloudflare Tunnel + Access for external reach). Two systemd user services:

- `hangar-supervisor.service` — owns PTYs, always on
- `hangar.service` — the `hangard` binary, can be restarted freely for updates

In dev, both can be launched via `cargo run --release --bin hangar-supervisor` and `cargo run --release --bin hangard` in separate tmux panes.

## What lives outside this diagram

- Migrations (`backend/migrations/`) — sqlx, idempotent
- Tests — unit tests alongside modules; integration tests in `backend/tests/`
- Logs aggregation (Phase 3 `/logs`) — separate concern, uses the same event bus plumbing
- Sandbox internals — see `docs/phases/06-sandboxing.md`
- Supervisor protocol details — see `docs/decisions/0010-sessions-survive-restart.md`
