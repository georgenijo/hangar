# ADR-0015: MVP bundles backend + UI + push + REST API into one release

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2

## Context

The natural decomposition is: (a) backend + UI, (b) push, (c) REST API. We could ship them separately, but the product isn't useful without all three.

## Decision

Phase 2 = one release that includes:
- Rust backend (PTY-owned sessions, SQLite + ring buffer, event bus)
- SvelteKit dashboard (chat UI + grid)
- Push via ntfy
- REST prompt API

Phases 3+ strictly add new features.

## Consequences

- Good: one "I use this daily now" milestone; no half-done dashboard period
- Good: integration testing happens once, not three times
- Bad: Phase 2 is long; several-week build before the first user-facing release
- Bad: if a sub-component takes longer than estimated, the whole release slips

## Future work

- Mitigation: Phase 2 is internally broken into 8 milestones, each shippable behind a feature flag or left on a branch. If any milestone delays, the prior seven are still deployable to a staging node.
