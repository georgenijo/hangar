# ADR-0007: Session slug is unique per node, not globally

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2 (spike for multi-node in Phase 10+)

## Context

Sessions have human-friendly `slug`s (like `wave`). Two slug uniqueness policies: per-node (slug is unique within a single host) or global (slug is unique across all hosts that ever run hangar).

## Options considered

1. **Per-node uniqueness.** `optiplex:wave` and `laptop:wave` can coexist.
2. **Global uniqueness.** Only one `wave` may exist at a time, anywhere.
3. **Defer entirely (UUID-only).** Avoid slug collisions by not exposing slugs.

## Decision

Option 1 for MVP. Spike research item filed for Option 2 when multi-node lands.

## Consequences

- Good: no coordination step at session creation
- Good: multi-node migration can namespace slugs as `<node>/<slug>` later
- Bad: dashboard must disambiguate when multi-node shows two nodes with the same slug (Phase 10 UI concern)
- Bad: URL sharing across nodes needs a node prefix in Phase 10 (planned in ROADMAP)

## Future work

- Spike: global slug service with TTL + conflict resolution (Phase 10 prerequisite)
