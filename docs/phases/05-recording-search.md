# Phase 5 — Recording + search

**Status:** ⬜ Planned

## Goal

Make every session's history searchable and replayable. Add an asciinema-style scrubber. Enable "find the plan Claude wrote 2 h ago" across all sessions.

## Non-goals

- No semantic / embedding search yet — exact + regex search only
- No cross-session summarization

## Deliverables

- Full-text index over events + output (SQLite FTS5)
- `GET /api/v1/search?q=...&session_ids=...&kinds=...`
- `/session/:id/replay` UI: timeline scrubber, play/pause, speed control
- Optional: per-turn anchors (jump to turn N)

## Acceptance criteria

- Searching a phrase returns results across all sessions in < 500 ms for 1 GB of history
- Replay scrubber plays back terminal output accurately, including ANSI
- Replay supports jumping to specific `AgentEvent`s (turn boundaries)

## Dependencies

- Phase 2 shipped with ring-buffer + events persistence

## Risks / unknowns

- FTS5 size growth vs pruning policy
- Replay fidelity with complex ANSI (alternate screen, resize events) — may need event-based replay instead of pure byte replay

## Estimated effort

2–3 Claude-sessions of work.
