# ADR-0010: Sessions survive backend restart via supervisor pattern

**Status:** Accepted
**Date:** 2026-04-17
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
