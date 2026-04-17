# Phase 8 — Hooks + policy engine

**Status:** 💭 Aspirational

## Goal

User-defined scripts run around every session event: inject context `before_prompt`, summarize `after_response`, gate `on_tool_use`. A policy DSL enforces guardrails like "this session cannot write outside `~/Documents/whoop`".

## Deliverables (direction)

- Scripting engine (candidate: `mlua` for Lua, `starlark-rust` for Starlark — ADR)
- Standard hook points: `before_prompt`, `after_response`, `on_tool_use`, `on_state_change`, `on_exit`
- Policy DSL with compile-time checks for common predicates (path, network host, tool name)
- Per-session policy attachments via labels or explicit API

## Acceptance criteria (draft)

- A Lua hook can intercept and modify an outgoing prompt before it reaches Claude
- A policy denial on `write_file("/etc/passwd")` surfaces as an `AgentEvent::ToolDenied`
- Hooks can push messages to external systems (ntfy, webhooks)

## Open questions

- Lua vs Starlark vs WASM plugins — ADR
- Sandboxing the hook interpreter itself
- Performance budget per hook (<10 ms p99)
