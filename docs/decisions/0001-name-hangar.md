# ADR-0001: Name the project "hangar"

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** All

## Context

The project needs a name before the first GitHub repo. Originally discussed as "optiplex-dashboard" — fine for a box-tied tool but limiting once the scope grows to a full agent OS that's potentially multi-node or open-sourced.

## Options considered

1. `optiplex-dashboard` — original placeholder. Couples the name to the box.
2. `paddock` — evokes "where agents run/race." Short, unique. No prior art conflict.
3. `agentos` — descriptive, but names like this are taken by multiple unrelated GitHub repos.
4. `corral` — similar vibe to paddock.
5. `hangar` — where agents are "parked" and maintained. Calm, mechanical, evocative of many agents with individual bays.

## Decision

**`hangar`**. Evocative, unique enough, aviation-spelling chosen deliberately.

## Consequences

- Good: fits the mental model of "many agents in individual bays with a control tower"
- Good: doesn't tie the name to a single machine
- Bad: people may mis-spell as "hanger" (the clothes one) — acceptable

## Future work

- Domain selection for Phase 1 (not decided)
- License choice (private for now)
