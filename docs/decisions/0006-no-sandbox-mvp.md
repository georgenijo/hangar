# ADR-0006: No sandboxing in the MVP

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2 (deferred to Phase 6)

## Context

Agents (Claude Code, Codex, shells) run arbitrary code. Proper isolation uses containers + overlayfs + network policy. Building this inside Phase 2 would roughly double the scope and block laptop development (Linux-only features).

## Options considered

1. **Phase 2 ships without sandboxing.** Agents run as user `george`. Trust boundary is the Tailnet and (Phase 1+) Cloudflare Access.
2. **Phase 2 builds sandboxing from day one.** Every session in podman + overlayfs + nftables.
3. **Never sandbox.** Rely on trust forever.

## Decision

Option 1. Ship Phase 2 without sandboxing; Phase 6 introduces it.

## Consequences

- Good: Phase 2 ships faster and stays reviewable
- Good: can develop + test on laptop (macOS) without Linux-specific containers
- Bad: an agent that misbehaves can do anything George can — acceptable in single-user, trust-boundary-is-Tailnet model
- Bad: Phase 6 will need to retrofit session configs + startup paths

## Future work

- Phase 6 introduces `SessionConfig.sandbox: Option<SandboxSpec>` without breaking existing sessions.
