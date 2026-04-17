# Phase 7 — Branching + snapshots

**Status:** 💭 Aspirational

## Goal

Treat a Claude Code conversation like a git branch. Snapshot a session mid-conversation, fork it, run an alternate path, compare outcomes.

## Deliverables (direction)

- `POST /api/v1/sessions/:id/snapshot` → captures tokens, overlay fs state, cwd, env, last turn id
- `POST /api/v1/sessions/:id/fork` → new session seeded from snapshot
- `Session.parent_id` field re-introduced (deferred from Phase 2)
- UI: branch tree view per session
- Optional: merge — cherry-pick a fork's final turn back into parent

## Acceptance criteria (draft)

- Forking a Claude session produces a new session with identical state in < 2 s
- Parent session is unaffected
- Tree view shows parent/child relationships

## Open questions

- How to capture Claude Code's live context exactly (tokens + conversation id + model)
- Whether fs overlay from Phase 6 is a prerequisite (likely yes)
- Merge semantics — cherry-pick a response vs. re-apply user prompts

Promote to ⬜ Planned after Phase 6 ships and the snapshot primitives are available.
