# Phase 9 — Inter-agent protocols

**Status:** 💭 Aspirational

## Goal

One session can programmatically ask another: "solve this subproblem, report back." hangar becomes a lightweight multi-agent scheduler.

## Deliverables (direction)

- `POST /api/v1/sessions/:id/task` — submit a task, await completion
- Task status: queued / running / succeeded / failed
- Routing: task describes required session kind + labels; hangar picks or spawns a worker
- UI: job tree per parent session, per-task logs, retry

## Acceptance criteria (draft)

- Session `wave` (Claude) can call `codex.solve({problem})` and receive a structured result
- Job tree in UI shows parent/child task relationships
- Failed subtasks surface with error + retry button

## Open questions

- Protocol shape (REST? JSON-RPC over ws?)
- Timeout + cancellation semantics
- Should agents themselves know they're delegating, or is it invisible plumbing?
