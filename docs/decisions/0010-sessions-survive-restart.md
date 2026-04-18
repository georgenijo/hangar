# ADR-0010: Sessions survive backend restart via supervisor pattern

**Status:** Accepted + Implemented
**Date:** 2026-04-17
**Implemented:** 2026-04-18 (branch `feat/supervisor-rollout`, parent issue #8)
**Phase:** 2

## Context

Backend restarts (code updates, crashes, config reloads) shouldn't kill long-running Claude sessions. We need a way for PTY child processes to outlive the backend process.

## Options considered

1. **Separate supervisor daemon.** A small always-running process holds PTY fds. Backend connects to it over a Unix socket, hands off fds, reads/writes. On backend restart, supervisor still owns the PTYs; backend reconnects.
2. **Systemd socket activation + re-parenting.** Backend runs under systemd with socket activation so a restart doesn't break listeners. PTY fds can be passed via systemd's `sd_notify` machinery or between restarts via `CLONE_NEWPID` tricks. Complex.
3. **Sessions die with backend.** Simplest; worst UX for long Claude runs.
4. **Per-session choice.** Flag on Session: `durable` or `ephemeral`. Same supervisor logic, applied selectively.

## Decision

Option 1 — separate supervisor daemon. Phase 2 implements a tiny `hangar-supervisor` process that:
- Holds PTY fds + child pids in memory
- Exposes a Unix socket on `~/.local/state/hangar/supervisor.sock`
- Accepts `spawn`, `attach_fd`, `resize`, `kill`, `list` commands
- Re-parents children it spawns so they survive supervisor restarts too (via `prctl(PR_SET_CHILD_SUBREAPER)` or double-fork)

Backend connects at startup, lists existing sessions, resumes event streams.

## Consequences

- Good: sessions survive both backend and supervisor restarts (under different conditions)
- Good: backend can be rapidly restarted during development without losing Claude context
- Good: testable in isolation (supervisor has its own test surface)
- Bad: +1 process to manage (extra systemd unit)
- Bad: fd passing over Unix sockets is ~200 LOC of careful code; needs good tests

## Future work

- Phase 2 spike: prototype supervisor first, validate fd handoff across backend restart before committing to full Phase 2 scope.

## Implementation notes (2026-04-18)

Landed behind #8 (rollout tickets #35 #36 #37 #38 #39).

- Binary at `backend/src/bin/hangar-supervisor.rs`; protocol types in `backend/src/supervisor_protocol.rs`; client in `backend/src/supervisor_client.rs`.
- Child-reaping path: `prctl(PR_SET_CHILD_SUBREAPER)` + an async `waitpid` loop. `KillMode=process` on the systemd unit so stopping the supervisor doesn't cascade-kill PTY children.
- Runs as a **user** systemd unit (`systemd/hangar-supervisor.service`), not system-wide — socket lives at `$XDG_STATE_HOME/hangar/supervisor.sock` under the invoking user. Install runbook: [`docs/runbook/supervisor-install.md`](../runbook/supervisor-install.md).
- Path/socket resolution is env-overridable via `HANGAR_STATE_DIR` + `HANGAR_SUPERVISOR_SOCK` (added for the integration test, also useful for second-instance dev).
- On dev boxes, `hangar-supervisor.service` is enabled but `hangar.service` stays disabled because `hangard` is usually run manually from a tmux pane. Prod enables both.
- Restart survival verified by `backend/tests/supervisor_restart.rs` (#37) and live on the optiplex box (#36 smoke run).

Divergence from the ADR body: nothing material. The supervisor uses subreaper-only (no double-fork) because it's already the direct parent, and the parent-outlives-supervisor case is handled by systemd re-spawning the supervisor with `Restart=always`.
