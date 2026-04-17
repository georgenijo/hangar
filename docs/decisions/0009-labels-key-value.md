# ADR-0009: Labels are key=value maps, not word lists

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2

## Context

Sessions carry user-defined tags for filtering (`project=whoop`, `environment=dev`, `goal=refactor`). Two shapes: key/value map vs. list of words.

## Options considered

1. **Key=value map.** `{"project": "whoop", "goal": "refactor"}`.
2. **List of words.** `["whoop", "refactor"]`.
3. **Both, with key=value wrapping a list.** Over-engineered.

## Decision

Option 1. Labels are `BTreeMap<String, String>`.

## Consequences

- Good: queries like "all sessions with `project=whoop`" are unambiguous
- Good: UI can show tags as pill chips grouped by key
- Bad: slightly more friction to add a tag (pick a key first)
- Bad: migration to list-of-words would be costly if regretted — accepting risk

## Future work

- Phase 2 UI includes a label editor in the session detail panel.
