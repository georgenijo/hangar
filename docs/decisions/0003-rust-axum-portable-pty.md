# ADR-0003: Rust + axum + portable-pty for the backend

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2

## Context

Backend language and HTTP framework choice for Phase 2.

## Options considered

1. **Rust + axum + tokio + portable-pty.** Tight binary, proven stack, portable-pty is the Rust de-facto PTY crate.
2. **Go + stdlib net/http + creack/pty.** Simple, low friction, acceptable performance, smaller community for complex async patterns.
3. **Node/TypeScript + Fastify + node-pty.** Fastest iteration but leaks RAM, CPU-heavy under load, node-pty is battle-tested but not as clean as portable-pty for headless scenarios.
4. **Python + FastAPI + ptyprocess.** Quick to prototype, but GIL + perf issues for streaming PTY bytes at scale.

## Decision

Option 1: Rust + axum + tokio + portable-pty + tower-http.

## Consequences

- Good: single static binary, systemd-friendly
- Good: memory and CPU predictable under many concurrent sessions
- Good: `portable-pty` gives us macOS/Linux PTY portability from day one (enables laptop dev + Phase 10 multi-node)
- Good: type system catches cross-module bugs before runtime
- Bad: slower iteration vs JS/Python during early prototyping
- Bad: slower cold `cargo build --release` on the i5-7500T box (~minutes); mitigated by `sccache` and incremental builds

## Future work

- ADR on supervisor/reattach strategy (ADR-0010)
- ADR on frontend stack (ADR-0005)
