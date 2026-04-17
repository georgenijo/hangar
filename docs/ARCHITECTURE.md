# hangar — Architecture

One-page system design. Written for humans and for AI agents working in this repo.

---

## Elevator pitch

hangar is a single-binary Rust backend that owns terminal sessions (PTYs), wraps known agents (Claude Code, Codex, shell) with smart drivers, and exposes a REST + WebSocket API. A SvelteKit frontend renders a command-center dashboard. The stack runs on a dedicated box reachable over Tailscale today and via Cloudflare Tunnel with SSO later.

---

## High-level diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      clients                                    │
│  laptop browser   phone (Safari/PWA)   iOS Shortcuts   cron     │
└──────────────┬──────────────┬──────────────┬────────────────────┘
               │              │              │
               ▼              ▼              ▼
    ┌─────────────────────────────────────────────┐
    │        Cloudflare Tunnel (Phase 1+)         │  public https + Access SSO
    │        │ bypassed on tailnet for LAN        │
    └──────────────────────┬──────────────────────┘
                           │
                   ┌───────▼────────┐
                   │  caddy :8080   │  tls, static, reverse-proxy
                   └───────┬────────┘
             ┌─────────────┼──────────────────┐
             ▼             ▼                  ▼
      /          /api/*  /ws/*         /s/* (legacy, Phase 0 ttyd view)
      │           │       │             │
      │           ▼       ▼             ▼
      │    ┌──────────────────────┐  ┌──────────────┐
      │    │  hangar backend      │  │ ttyd procs   │
      │    │  (Rust, axum)        │  │ per session  │
      │    │  :3000 localhost     │  │ (retired in  │
      │    │                      │  │  Phase 2)    │
      │    │  - session registry  │  └──────────────┘
      │    │  - pty supervisor    │
      │    │  - agent drivers     │
      │    │  - event bus         │
      │    │  - sqlite + ringbuf  │
      │    │  - push rules        │
      │    └──────────┬───────────┘
      │               │
      │    ┌──────────┴──────────┐
      │    ▼                     ▼
      │  pty processes      SQLite + ring files
      │  (Claude, Codex,    ~/.local/state/hangar/
      │   shells)
      │
      ▼
  svelte dashboard (SPA)
```

---

## Component boundaries

| Component | Responsibility |
|---|---|
| **caddy** | TLS termination (Phase 1+), static file serving, reverse-proxy to backend, tunnel to Cloudflare |
| **backend (Rust)** | Session lifecycle, PTY management, agent driver traits, event bus, API surface, SQLite + ring file persistence, push rule evaluation |
| **agent drivers** | Per-kind adapter (shell, claude-code, raw-bytes). Spawn, parse, state-detect, shutdown |
| **frontend (SvelteKit)** | Command-center UI, session tiles, chat view for Claude, grid view, filters, logs viewer |
| **push dispatcher** | Watches event bus for trigger rules (`awaiting_permission`, `error`, `idle→active`), fans out to ntfy/APNs |
| **tmux** | Remains as an ad-hoc terminal multiplexer George uses interactively over SSH. Not in hangar's path for Phase 2+ |

Rule: the backend is the only component that talks to PTYs. Everything else goes through its API.

---

## Data model (core types)

See [`SESSION-PROTOCOL.md`](SESSION-PROTOCOL.md) for full spec. Summary:

```rust
struct Session {
    id: SessionId,              // ULID, sortable, opaque
    slug: String,               // "wave" — unique per node
    kind: SessionKind,          // Shell | ClaudeCode | RawBytes { command }
    state: SessionState,        // Booting | Idle | Streaming | Awaiting | Error | Exited
    cwd: PathBuf,
    env: BTreeMap<String, String>,
    agent_meta: Option<AgentMeta>,   // model, version, tokens, …
    labels: BTreeMap<String, String>, // free-form k=v tags
    node_id: NodeId,                  // reserved for multi-node (Phase 10+)
    pty: PtyHandle,                   // internal, not serialized
    created_at: Timestamp,
    last_activity_at: Timestamp,
}
```

Events (persisted):

```rust
enum Event {
    SessionCreated(Session),
    StateChanged { id: SessionId, from: SessionState, to: SessionState },
    OutputAppended { id: SessionId, offset: u64, len: u32 },  // pointer into ring file
    AgentEvent { id: SessionId, event: AgentEventKind },       // turn, tool_call, thinking, …
    Exited { id: SessionId, code: Option<i32> },
}
```

---

## Persistence

- **SQLite** at `~/.local/state/hangar/hangar.db`
  - `sessions` table (metadata + current state)
  - `events` table (append-only log, indexed by session_id + ts)
  - `labels` index for tag queries
- **Ring-buffer files** at `~/.local/state/hangar/sessions/<id>/output.bin`
  - Raw PTY byte stream, rotates at 100 MB per session
  - Event log stores byte offsets, not bytes themselves
- **Config** at `~/.config/hangar/config.toml`
  - Push rules, default shell, agent driver settings

Backups: add these paths to the box's existing restic job.

---

## Process model

Single Rust binary `hangard` runs under systemd as user `george`. Responsibilities:

1. **HTTP server** (axum + tokio) on `localhost:3000`
2. **PTY supervisor**: spawns child processes, streams I/O, monitors exit
3. **Agent drivers** run as async tasks per session, parsing output, emitting structured events
4. **Event bus**: `tokio::sync::broadcast` channel fan-out to WebSocket subscribers + persistent log writer + push dispatcher
5. **Metrics + logs**: logs to stdout (systemd captures to journald), metrics at `/api/metrics`

**Crash recovery**: on restart, backend scans SQLite for sessions in non-terminal states. A small supervisor daemon (or systemd-managed re-parenting) holds PTY fds via Unix socket so sessions survive backend restarts. See [ADR-0010](decisions/0010-sessions-survive-restart.md).

---

## Agent drivers

Trait (abridged, full in [`SESSION-PROTOCOL.md`](SESSION-PROTOCOL.md)):

```rust
trait AgentDriver: Send + Sync {
    fn kind(&self) -> SessionKind;
    fn spawn(&self, cfg: &SessionConfig) -> Result<PtyHandle>;
    fn on_bytes(&mut self, bytes: &[u8]) -> Vec<AgentEvent>;  // parse
    fn detect_state(&self, ctx: &StateCtx) -> Option<SessionState>;
    fn shutdown(&self, handle: &PtyHandle) -> Result<()>;
}
```

Phase 2 drivers:
- `ShellDriver` — raw bytes in/out, state = idle/busy heuristic on cursor position
- `ClaudeCodeDriver` — parses transcript patterns + consumes hook events via a Unix socket, produces structured `AgentEvent`s (turn_started, turn_finished, tool_call, thinking_block, awaiting_permission, tokens_used, model_changed)
- `RawBytesDriver` — fallback: no parsing, all state is `Streaming`/`Idle` via activity timer

---

## Security & trust model (MVP)

| Zone | Posture |
|---|---|
| Code running inside sessions | Trusted (same user as backend, no sandbox) |
| Network access (Phase 0/1) | Tailnet only — no auth on backend |
| Network access (Phase 1 public tunnel) | Cloudflare Access SSO gate (email allowlist) |
| Push notifications | Outbound only, no incoming webhooks |
| Session control (prompt/kill) | Whoever reaches the API can issue any command |

No sandboxing for MVP. Agents run as user `george` with full access to the box. Acceptable because:
1. Only George has tailnet access today
2. Cloudflare Access gates external access
3. Sandbox is a real future phase, not a permanent choice

See [ADR-0006](decisions/0006-no-sandbox-mvp.md) and the sandbox phase doc.

---

## Failure modes

| Failure | Behavior |
|---|---|
| Backend crash | Sessions survive via supervisor; clients reconnect after restart; no data loss (events persisted) |
| Disk full | Ring buffers cap at 100 MB, rotate. SQLite growth monitored. Push alert at 80% disk |
| Agent hangs | State detector flags `Idle` after silence threshold; `Awaiting` if prompt patterns match |
| PTY child crash | Event `Exited`, state `Error` or `Exited`, no auto-restart (manual respawn) |
| Push dispatcher failure | Events still persist, dashboard still works; only notifications lost |
| Cloudflare Tunnel down | Tailnet access unaffected; phone falls back to Tailscale (if installed) |

---

## Environment

- **Host**: Dell OptiPlex 7050, i5-7500T, 8 GB RAM, 500 GB HDD, Ubuntu 24.04
- **Tailnet hostname**: `optiplex` (IP 100.103.161.109)
- **User**: `george` (passwordless sudo)
- **Existing services**: tmux `work` session, cockpit on `:9090`, ttyd on `:7681`, restic backups
- **New services** (Phase 0+): ttyd-codex/wave/issue12 systemd units, caddy on `:8080`

---

## What this doc is not

- Not a migration plan — see [`PHASES.md`](PHASES.md)
- Not a spec for agent drivers — see [`SESSION-PROTOCOL.md`](SESSION-PROTOCOL.md)
- Not a rationale log — see [`decisions/`](decisions)

If a question isn't answered here, check the phase doc for the work currently underway. If a decision feels ambiguous, raise it as an ADR before coding around it.
