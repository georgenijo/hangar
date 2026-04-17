# ADR-0011: Ring-buffer files for raw bytes, SQLite for events

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2

## Context

See ADR-0004 for the macro decision. This ADR pins the mechanics.

## Decision

- Each session gets `~/.local/state/hangar/sessions/<id>/output.bin` — a fixed-size ring buffer (100 MB default, see ADR-0012).
- File header: 16 bytes — `magic(4) | version(1) | pad(3) | head(u64)`. `head` is the next-write offset modulo capacity.
- Writes append at `head`, wrapping at capacity. Never rewrite the header outside the 16-byte region.
- Structured events go to SQLite `events` table. `OutputAppended` events carry `(offset, len)` referring to the ring file.
- Readers compute `(offset, len) → file region`, handle wrap by reading two segments.

## Consequences

- Good: append-only writes are fast; no fragmentation
- Good: bounded storage per session without LRU logic in the event log
- Good: events DB stays compact (MB, not GB)
- Bad: consumers (replay UI) need to handle wrap-around correctly
- Bad: once data wraps, old events' `(offset, len)` references are stale. Events older than the ring capacity are kept in the DB (state transitions, agent events) but their raw-bytes context is gone.

## Future work

- Phase 5 (recording + search) adds an option to archive expired ring segments elsewhere before they're overwritten.
