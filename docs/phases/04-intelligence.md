# Phase 4 — Deeper intelligence + Codex driver

**Status:** ✅ Shipped (2026-05-13) — Codex driver deferred to [#112](https://github.com/georgenijo/hangar/issues/112)

## Goal

Expand what the dashboard knows about each session: add a proper Codex driver (matching Claude Code's feature set), tune state detection so "stuck", "idle", "busy" are accurate, and surface per-session insights (recent tool calls, context-window warnings, cost-so-far).

## Non-goals

- No new session types beyond Claude Code and Codex — custom agents remain RawBytes
- Not a full observability stack

## Deliverables

- ~~`drivers/codex.rs`~~ — deferred to #112; stub exists but patterns unverified without real output fixtures
- ✅ `ClaudeCodeDriver` improved — COMPACT_RE no longer emits misleading zeros; SUBAGENT_RE and THINK_BUDGET_RE guarded with `!hooks_active` to prevent hook/PTY duplicate events
- ✅ `StateCtx` activity histograms — `event_timestamps` ring (last 20) wired into `detect_state`; hung-session detection transitions Streaming → Idle after 90s dual-silence (bytes + events)
- ✅ Per-session insights panel — `InsightsPanel.svelte`: live CTX% bar, cost estimate, last 5 tool calls
- ✅ Alert rules — `approaching_context_window` push rule fires once per session at 80%; `high_token_burn`, `context_window_80pct` also shipped
- ✅ CommandView (`/command`) — KPI row, 7-day spend bar chart, active sessions list, pipeline runs, host gauges
- ✅ FleetView (`/fleet`) — localhost host card with ring gauges, sessions count, remote-hosts empty state

## Acceptance criteria

- ~~A running Codex session in the dashboard shows turn bubbles identical in structure to Claude sessions~~ — deferred to #112
- ✅ State detector distinguishes `Idle` from `Streaming` correctly; hung sessions (>90s silence) auto-recover to Idle
- ✅ Per-session insights panel shows live context-window %, today's cost, last 5 tool calls
- ✅ Push rule "approaching context window" fires once per session at 80 %

## Dependencies

- Phase 2 shipped ✅
- Codex CLI version used must be pinned and its output format captured in fixtures (pending #112)

## Risks / unknowns

- Codex output format may change; fixture-based tests catch regressions (pending #112)
- Cost estimation requires a per-model price table kept up-to-date

## Estimated effort

2 Claude-sessions of work.
