# ADR-0014: Dashboard visibility is the killer feature

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** All

## Context

George's stated top need is visibility: seeing what every running agent is doing, at a glance, from any device. Push notifications, REST API, search — all important — rank below visibility.

## Decision

Prioritize the command-center dashboard as the "ship quality" bar. Other features are additive.

## Consequences

- Every phase's acceptance criteria must advance dashboard usefulness
- When trade-offs arise (feature vs. UI polish), UI wins at MVP stage
- Phases whose only value is plumbing (e.g. recording storage) must justify how they enable visibility gains

## Future work

- Frontend gets disproportionate effort in Phase 2 vs. a "shell-first" tool would
- Post-MVP: consider a dedicated "dashboard sprint" (UI polish week) if the first version ships rough
