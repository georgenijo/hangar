# Phase 4 — Deeper intelligence + Codex driver

**Status:** ⬜ Planned

## Goal

Expand what the dashboard knows about each session: add a proper Codex driver (matching Claude Code's feature set), tune state detection so "stuck", "idle", "busy" are accurate, and surface per-session insights (recent tool calls, context-window warnings, cost-so-far).

## Non-goals

- No new session types beyond Claude Code and Codex — custom agents remain RawBytes
- Not a full observability stack

## Deliverables

- `drivers/codex.rs` — parses Codex CLI output format, emits `AgentEvent`s equivalent to Claude's
- Improved `ClaudeCodeDriver` — more patterns: compaction warnings, subagent spawning, thinking-budget signals
- `StateCtx` enriched with activity histograms to distinguish "agent thinking" from "agent hung"
- Per-session insights panel in frontend: recent tools, cost estimate, context-window usage chart
- Alert rules expanded: high token burn, approaching context window, repeated errors

## Acceptance criteria

- A running Codex session in the dashboard shows turn bubbles identical in structure to Claude sessions
- State detector distinguishes `Idle` from `Streaming` correctly on ≥95 % of real turns measured over a week
- Per-session insights panel shows live context-window %, today's cost, last 5 tool calls
- Push rule "approaching context window" fires once per session at 80 %

## Dependencies

- Phase 2 shipped
- Codex CLI version used must be pinned and its output format captured in fixtures

## Risks / unknowns

- Codex output format may change; fixture-based tests catch regressions
- Cost estimation requires a per-model price table kept up-to-date

## Estimated effort

2 Claude-sessions of work.
