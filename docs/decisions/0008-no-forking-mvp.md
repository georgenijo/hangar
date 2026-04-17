# ADR-0008: No forking / branching in MVP

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2 (deferred to Phase 7)

## Context

Forking a session (save-state + copy + run alternate path) is attractive but expensive to build right. It requires snapshotting Claude Code's internal conversation state, fs overlay, environment, and tokens.

## Options considered

1. **Skip entirely for MVP.** No `parent_id` field on Session; add later when needed.
2. **Lightweight: metadata-only parent pointer but no actual snapshot plumbing.**
3. **Full forking in Phase 2.**

## Decision

Option 1. Drop `parent_id` from the Phase 2 schema. Re-introduce in Phase 7 when snapshots are supported.

## Consequences

- Good: smaller Phase 2 scope, cleaner schema
- Good: avoids half-finished feature on disk
- Bad: Phase 7 requires a schema migration to add `parent_id` + snapshot blob references
- Bad: George said he didn't know yet if forking would be useful — we get signal before committing to it

## Future work

- Phase 7 (aspirational) fleshes out snapshot + fork semantics
