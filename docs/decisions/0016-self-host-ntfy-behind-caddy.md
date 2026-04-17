# ADR-0016: Self-host ntfy behind Caddy

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 2.7

## Context

Push notifications to George's phone require an ntfy server to relay events from the backend to iOS via APNs. The hangar backend runs on a single Optiplex box reachable via Cloudflare Tunnel (`optiplex.<domain>`) per ADR-0013. The box already runs Caddy as a reverse proxy. We need a notification channel for four event types: permission prompts, agent errors, session exits, and context window threshold.

Constraints:
- Single-box deployment — no additional infra budget
- Caddy already handles reverse-proxying; adding a block is trivial
- ADR-0013: tailnet is the auth boundary — no per-endpoint auth needed for MVP
- iOS ntfy app relays to APNs via ntfy's upstream relay (UP protocol) — no Apple Developer account required

## Options considered

1. **ntfy.sh hosted** — zero ops, well-maintained. Trade-offs: external dependency (ntfy.sh outages break push), ~100ms LAN→internet→LAN round-trip for POST, topic names visible to ntfy.sh operator, free tier rate limits (250 msgs/day), no SLA.

2. **Self-hosted ntfy on Optiplex behind Caddy** — full control, sub-5ms POST latency (loopback), topic secret stays on box, no rate limits, one more systemd unit to manage. ntfy upstream relay (ntfy.sh) still needed for APNs delivery — this is ntfy's architecture, not a dependency we can avoid without an Apple Developer account.

3. **Custom WebSocket push to iOS app** — maximum control, no third-party relay. Trade-offs: requires building and maintaining an iOS app, APNs provisioning, app store distribution. Massive scope creep for a single-operator tool.

## Decision

Self-host ntfy. Run as a systemd unit (`ntfy.service`), Caddy proxies `handle_path /ntfy/*` to ntfy's HTTP port (default 2586). Topic name is the shared secret — knowledge of the topic name authorizes subscription. ntfy's iOS app handles APNs relay transparently via the UP protocol to ntfy.sh upstream; this requires no configuration beyond the default `ntfy serve`.

## Consequences

- Good: sub-5ms event-to-POST latency (loopback); no external POST dependency; topic secret stays on the box; no rate limits; standard systemd restart/monitoring
- Bad: one more service to monitor; ntfy upstream relay (ntfy.sh) is still required for iOS APNs delivery — if ntfy.sh is down, iOS push fails (ntfy web UI continues to work via self-hosted server)
- Future work opened up: can add ntfy access control (user/pass or token auth) later if the tailnet boundary changes; can add ntfy email or other notification channels without code changes
