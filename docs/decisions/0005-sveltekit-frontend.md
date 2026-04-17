# ADR-0005: SvelteKit for the frontend from day one

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2

## Context

The Phase 0 dashboard is plain HTML + vanilla JS. Phase 2 wants a real product UI (chat view for Claude, grid view, live badges, future reactive state). That needs a framework.

## Options considered

1. **Plain HTML/JS through Phase 2.** Keep it simple, upgrade later.
2. **SvelteKit.** Compiler-based, small bundles, idiomatic reactive stores, file-routing, works as SPA.
3. **Solid + Vite.** Tiny, reactive, less ecosystem.
4. **Astro + islands.** Great for content sites, awkward for app-like UIs.
5. **Next.js / React.** Big ecosystem, heavier runtime, SSR unnecessary for authenticated SPA.

## Decision

**SvelteKit** in SPA mode (no SSR). Caddy serves the built static output.

## Consequences

- Good: small runtime, fast updates with `svelte/store` for live event streams
- Good: file-based routing simplifies navigation
- Good: first-class TypeScript
- Good: chat-UI components (bubbles, collapsible) are easy to compose
- Bad: another toolchain on the box (pnpm, node); installable in minutes
- Bad: team of 1 means the Svelte ecosystem needs to stay simple; avoid premature abstraction

## Future work

- Component library choice (skeleton.dev? shadcn-svelte?) deferred to first Phase 2 frontend PR
