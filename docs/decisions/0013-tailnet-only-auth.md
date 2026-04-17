# ADR-0013: No auth for MVP — Tailnet is the boundary

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2 (Cloudflare Access added in Phase 1 for public URL)

## Context

Hangar's HTTP surface is reachable from within the Tailscale network today and (Phase 1) via Cloudflare Tunnel. Authentication can live at the transport layer, app layer, or both.

## Options considered

1. **No auth in backend.** Tailnet restricts access at the network layer; Cloudflare Access (Phase 1) restricts public access via SSO.
2. **Shared API token.** Header-based. Simple but annoying in browsers.
3. **Full per-user tokens + scopes.** Future-proof; overkill for single-user.

## Decision

Option 1. Backend accepts all requests from localhost-bound listener plus anything Caddy forwards. Trust boundary is Tailnet or Cloudflare Access.

## Consequences

- Good: zero auth code in Phase 2
- Good: no token rotation work
- Bad: anyone with Tailnet access can type into any session — acceptable in single-user model
- Bad: if Cloudflare Tunnel ever runs without Access gate, system is wide open. Mitigated by ADR-0001 domain + Access being mandatory.

## Future work

- Phase 6+ may introduce per-session tokens for read-only sharing (plumbing only, not auth-as-a-service).
- Multi-user auth is explicitly out of scope for the foreseeable future.
