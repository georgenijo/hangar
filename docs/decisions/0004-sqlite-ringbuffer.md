# ADR-0004: SQLite + ring-buffer files for persistence

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2

## Context

Where to store session metadata, structured events, and raw PTY output bytes.

## Options considered

1. **All in SQLite.** One database file holds metadata + events + raw terminal bytes as BLOBs.
2. **All in flat files.** Per-session files on disk, no DB.
3. **Hybrid: SQLite for metadata + events, ring-buffer files for raw bytes.**
4. **Postgres.** Future-proof for multi-node, overkill for single box.

## Decision

Option 3. SQLite for `sessions` + `events` tables. Raw PTY output streams to a ring-buffer file per session (100 MB cap, see ADR-0012). Events store `(offset, len)` pointers into that file rather than inlining bytes.

## Consequences

- Good: fastest write path for high-throughput byte streams (append-only file)
- Good: event table stays small enough for fast queries + FTS5 indexing (Phase 5)
- Good: ring files can be rotated without touching the DB
- Bad: two data locations to back up (mitigated — restic already covers the paths)
- Bad: crash recovery needs care (ring file head/tail must be flushed with matching event inserts; use WAL mode + `fsync` at event boundaries)

## Future work

- Phase 10 multi-node may move to Postgres or a distributed SQLite like `rqlite`; current schema designed to migrate cleanly.
