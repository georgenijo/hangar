# hangar — Session Protocol

The authoritative spec for how sessions are modelled, how agent drivers plug in, and what flows over the wire. This doc is for agents and humans writing backend code; the product-level "what is a session" explanation lives in [`ARCHITECTURE.md`](ARCHITECTURE.md).

Versioning: treat this doc as **v0** — subject to revision during Phase 2 build. Breaking changes require an ADR.

---

## Table of contents

1. [Session type](#session-type)
2. [Session state machine](#session-state-machine)
3. [Event types](#event-types)
4. [Agent driver trait](#agent-driver-trait)
5. [Claude Code hook wire format](#claude-code-hook-wire-format)
6. [HTTP API](#http-api)
7. [WebSocket channels](#websocket-channels)
8. [Persistence schema](#persistence-schema)
9. [Naming + IDs](#naming--ids)
10. [Failure semantics](#failure-semantics)

---

## Session type

```rust
pub struct Session {
    pub id: SessionId,                          // ULID (128-bit, sortable)
    pub slug: String,                           // unique per node, human-typable
    pub kind: SessionKind,
    pub state: SessionState,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,          // effective env at spawn
    pub agent_meta: Option<AgentMeta>,
    pub labels: BTreeMap<String, String>,       // user tags, k=v
    pub node_id: NodeId,                        // reserved for multi-node
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub exit: Option<ExitInfo>,
}

pub enum SessionKind {
    Shell { command: Vec<String> },             // ["/bin/bash", "-l"]
    ClaudeCode { config_override: Option<PathBuf>, project_dir: Option<PathBuf> },
    RawBytes { command: Vec<String> },          // anything else
}

pub struct AgentMeta {
    pub name: String,                           // "claude-code"
    pub version: Option<String>,                // "2.1.112"
    pub model: Option<String>,                  // "claude-opus-4-7[1m]"
    pub tokens_used: u64,
    pub last_tool_call: Option<String>,
}
```

Invariants:
- `slug` matches `^[a-z][a-z0-9-]{0,31}$`, unique per `node_id`
- `id` is the stable identifier in APIs; `slug` is for humans; URLs use both (`/session/<slug>-<short-id>`)
- `kind` is immutable once created
- `state` transitions follow the machine below

---

## Session state machine

```
         ┌─────────┐
         │ Booting │  ← spawn issued, PTY not yet active
         └────┬────┘
              │ agent producing output OR shell prompt visible
              ▼
         ┌─────────┐◀─────────────────┐
         │  Idle   │                  │
         └────┬────┘                  │ output stops for >Ns
              │                       │
              │ output resumes        │
              ▼                       │
         ┌──────────┐                 │
         │Streaming │─────────────────┘
         └────┬─────┘
              │ agent-specific "awaiting" pattern detected
              ▼
         ┌──────────┐
         │ Awaiting │  (permission prompt, input requested, etc.)
         └────┬─────┘
              │ user responds / timeout
              ▼
         ┌─────────┐         ┌───────┐
         │  Idle   │──error──▶│ Error │
         └────┬────┘         └───┬───┘
              │ pty exits         │ pty exits
              ▼                   ▼
         ┌──────────────────────────┐
         │        Exited            │
         └──────────────────────────┘
```

Legal transitions only (enforced by backend). Driver emits hints, backend decides.

---

## Event types

Every event is persisted in the `events` table and broadcast on the event bus.

```rust
pub enum Event {
    SessionCreated { session: Session },
    StateChanged { id: SessionId, from: SessionState, to: SessionState, reason: StateChangeReason },
    OutputAppended { id: SessionId, offset: u64, len: u32 },      // pointer into output.bin
    AgentEvent { id: SessionId, event: AgentEvent },
    MetadataUpdated { id: SessionId, patch: serde_json::Value },  // partial Session update
    Exited { id: SessionId, exit: ExitInfo },
    PushTriggered { id: SessionId, rule: String, channel: String },
}

pub enum AgentEvent {
    TurnStarted { turn_id: u64, role: TurnRole, content_start: Option<String> },
    TurnFinished { turn_id: u64, tokens_used: u32, duration_ms: u32 },
    ThinkingBlock { turn_id: u64, len_chars: u32 },
    ToolCallStarted { turn_id: u64, call_id: String, tool: String, args_preview: String },
    ToolCallFinished { turn_id: u64, call_id: String, ok: bool, result_preview: String },
    AwaitingPermission { tool: String, prompt: String },
    ModelChanged { model: String },
    Error { message: String },
    ContextWindowSizeChanged { pct_used: f32, tokens: u64 },
}
```

Serialization: JSON on the wire, rmp-serde (MessagePack) for persistent events to save space.

---

## Agent driver trait

```rust
pub trait AgentDriver: Send + Sync + 'static {
    fn kind(&self) -> &'static str;

    // Prepare a command + env for spawn. Backend creates the PTY.
    fn spawn_cfg(&self, req: &SpawnRequest) -> Result<SpawnCfg>;

    // Called on every new slice of PTY output bytes. Return any structured events
    // the driver was able to extract.
    fn on_bytes(&mut self, bytes: &[u8]) -> Vec<AgentEvent>;

    // Receive out-of-band events from the agent (e.g. Claude Code hook socket
    // messages). May emit AgentEvents too.
    fn on_oob(&mut self, msg: OobMessage) -> Vec<AgentEvent>;

    // Inspect recent state to suggest a SessionState transition. Backend decides
    // whether the transition is legal.
    fn detect_state(&self, ctx: &StateCtx) -> Option<SessionState>;

    // Deliver a prompt from the API. Default impl: write bytes to PTY.
    fn prompt(&self, handle: &PtyHandle, text: &str) -> Result<()> {
        handle.write_all(text.as_bytes())?;
        handle.write_all(b"\r")?;
        Ok(())
    }

    // Graceful shutdown.
    fn shutdown(&self, handle: &PtyHandle, grace: Duration) -> Result<()>;
}
```

Phase 2 driver list:

| Driver | Notes |
|---|---|
| `ShellDriver` | Spawns `$SHELL -l`. No structured parsing. State by activity timer + prompt-regex. |
| `ClaudeCodeDriver` | Spawns `claude` with a wrapped config that points hooks at a Unix socket. Parses known terminal patterns (context window box, model line, permission prompt glyph) as fallback when hooks are unavailable. Emits `AgentEvent`s. |
| `RawBytesDriver` | Generic: any command. No parsing beyond idle timer. |

New drivers land in their own module under `backend/src/drivers/` and register via a `DriverRegistry` at startup.

---

## Claude Code hook wire format

hangar configures Claude Code hooks in the user's `settings.json` (on box) to POST structured JSON to `http://localhost:3000/_cc_hook` on each event. The backend maps hook events to `AgentEvent`s.

Required hooks:

| Hook | Claude Code event | AgentEvent emitted |
|---|---|---|
| `SessionStart` | `claude` launches | `TurnStarted` w/ role=system (boot) |
| `UserPromptSubmit` | User sends prompt | `TurnStarted` role=user |
| `Stop` | Turn ends | `TurnFinished` |
| `PreToolUse` | Tool about to run | `ToolCallStarted` |
| `PostToolUse` | Tool finished | `ToolCallFinished` |
| `Notification` | Permission prompt / bell | `AwaitingPermission` |

Fallback: if hooks fail to register (old Claude Code, config drift), driver falls back to terminal pattern matching and logs a warning event.

Hook body shape:

```json
{
  "hangar_session_id": "01HV...",
  "hook": "PreToolUse",
  "ts": "2026-04-17T03:51:00Z",
  "payload": { ... Claude Code's native hook body ... }
}
```

Socket binding is localhost-only; no authentication (box is trust boundary).

---

## HTTP API

All responses JSON, versioned via `/api/v1/…` prefix.

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/v1/sessions` | List. Query params: `kind=`, `label.<k>=<v>`, `state=`. |
| POST | `/api/v1/sessions` | Spawn. Body: `{slug, kind, cwd?, env?, labels?}`. |
| GET | `/api/v1/sessions/:id` | Full session object. |
| PATCH | `/api/v1/sessions/:id` | Update labels, rename slug. |
| DELETE | `/api/v1/sessions/:id` | Shutdown + remove record. |
| POST | `/api/v1/sessions/:id/prompt` | Body: `{text}`. Send prompt. |
| POST | `/api/v1/sessions/:id/key` | Body: `{key}` (e.g. `"Ctrl-c"`). Send keystroke. |
| POST | `/api/v1/sessions/:id/resize` | Body: `{cols, rows}`. |
| GET | `/api/v1/sessions/:id/output` | Query `?offset=&len=`. Returns raw bytes from ring file. |
| GET | `/api/v1/sessions/:id/events` | Query `?since=&kind=`. Returns persisted events. |
| POST | `/api/v1/broadcast` | Body: `{text, filter}`. Send prompt to multiple sessions. |
| GET | `/api/v1/metrics` | JSON metrics blob. |
| GET | `/api/v1/health` | `{status, version, uptime_s}`. |

IDs accept both ULID (`01HV…`) and slug. When slug is given and is ambiguous (unlikely pre-multi-node), backend picks oldest.

---

## WebSocket channels

| Path | Purpose | Messages |
|---|---|---|
| `/ws/v1/events` | Global event bus | Server→client: `Event` JSON. Client→server: subscription filters. |
| `/ws/v1/sessions/:id/pty` | Live terminal | Binary frames for bytes, JSON frames for resize/ack. Replaces ttyd. |
| `/ws/v1/logs` | Host logs firehose (Phase 3) | Text frames with `{source, ts, level, line}`. |

WebSocket connections are automatically reconnected by the frontend. Server sends a snapshot on connect, then deltas.

---

## Persistence schema

SQLite schema (Phase 2 baseline):

```sql
CREATE TABLE sessions (
    id          TEXT PRIMARY KEY,       -- ULID
    slug        TEXT NOT NULL,
    node_id     TEXT NOT NULL DEFAULT 'local',
    kind        TEXT NOT NULL,          -- JSON-encoded SessionKind
    state       TEXT NOT NULL,          -- enum name
    cwd         TEXT NOT NULL,
    env         TEXT NOT NULL,          -- JSON
    agent_meta  TEXT,                   -- JSON
    labels      TEXT NOT NULL,          -- JSON object
    created_at  INTEGER NOT NULL,
    last_activity_at INTEGER NOT NULL,
    exit        TEXT,                   -- JSON
    UNIQUE (node_id, slug)
);

CREATE TABLE events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT NOT NULL REFERENCES sessions(id),
    ts          INTEGER NOT NULL,       -- ms since epoch
    kind        TEXT NOT NULL,
    body        BLOB NOT NULL           -- MessagePack Event
);
CREATE INDEX events_by_session ON events (session_id, ts);
CREATE INDEX events_by_kind    ON events (kind, ts);

CREATE TABLE metrics_daily (
    day         TEXT PRIMARY KEY,       -- YYYY-MM-DD
    tokens_total INTEGER NOT NULL,
    sessions_created INTEGER NOT NULL,
    push_sent   INTEGER NOT NULL
);
```

Output ring files live at `~/.local/state/hangar/sessions/<id>/output.bin`. Backend writes a ring-buffer struct: 8-byte header (`head`, `tail` offsets) + payload. Events carry `(offset, len)` into this file.

---

## Naming + IDs

- **`id`**: ULID, lowercase, 26 chars. Stable forever.
- **`slug`**: short human label. Must match `^[a-z][a-z0-9-]{0,31}$`.
- **URL form**: `/session/<slug>-<first 6 chars of id>` for human-friendly + stable-across-rename.
- **Node ID** (Phase 10+): `optiplex`, `laptop`, etc. Always `local` for Phase 2.
- **Shared tunnel hostname** (Phase 1+): `optiplex.georgenijo.com`.

---

## Failure semantics

| Scenario | Behavior | User-visible effect |
|---|---|---|
| Backend restart | Sessions survive (supervisor holds PTY fds). Events before restart are persisted. | Brief disconnect; client reconnects and refetches state. |
| Agent driver panic | Driver crash isolated, session marked `Error`, PTY continues with fallback raw-bytes handling. | Tile shows error badge, terminal still usable. |
| Ring file corrupted | Session's output history unreadable; events remain; output is truncated from last good offset. | Scrollback past the corruption is lost; future output works. |
| SQLite lock | Event write blocks briefly (WAL mode, mostly nonblocking). | No user impact under normal load. |
| Clock skew | Events may reorder slightly on replay. | Minor; timeline scrubber handles out-of-order. |
| Out of disk | Ring files refuse further writes; event persist fails; backend degrades to read-only and pushes a critical alert. | Dashboard shows disk-full banner. |

---

## Open questions (to resolve before Phase 2 ships)

- Should `PATCH /sessions/:id` allow `kind` transition (e.g. Shell → ClaudeCode when user types `claude` in a Shell session)? Tentative: no; upgrade by spawning a new session of the right kind.
- Hook socket authentication: tie to session id via a per-session HMAC, or trust localhost? Tentative: HMAC so stray hook bodies can't attribute to wrong session.
- Retention of events: cap per-session event count? Rotate with output? Tentative: cap at 50k events per session, roll oldest.
- Metrics format: per-session stats should be accessible at `/api/v1/sessions/:id/metrics` without scanning all events. Add a `sessions_stats` table? Deferred to Phase 3.

These convert to ADRs as decisions are made.
