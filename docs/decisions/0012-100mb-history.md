# ADR-0012: 100 MB output history cap per session

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2

## Context

Ring buffer size trade-off: bigger = more history retained; smaller = less disk per session.

## Options considered

- 10 MB — days of Claude work, negligible disk footprint even for 100 sessions.
- 100 MB — weeks of Claude work, 10 GB for 100 sessions on a 500 GB disk.
- Unlimited — never roll; risk filling disk.

## Decision

**100 MB default.** Configurable per-session in future phases.

## Consequences

- Good: enough scrollback for long multi-day Claude sessions
- Good: full-text search across a single session stays sub-second
- Good: total disk usage predictable
- Bad: not enough for sessions that want full lifetime history (Phase 5's archive strategy addresses this)

## Future work

- `SessionConfig.history_bytes: Option<u64>` — per-session override
- Phase 5 archives expired ring segments before wrap
