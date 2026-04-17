# Phase 1 — Cloudflare Tunnel + Access

**Status:** ⬜ Planned

## Goal

Expose the Phase 0 dashboard at a stable public URL so the phone reaches it without Tailscale. Add an SSO gate so only George can load it.

Concretely: `https://optiplex.<domain>/` works on cellular, from any browser, and every request goes through Cloudflare Access email allowlist (just `george.nijo8@gmail.com` for MVP).

## Non-goals

- No new features beyond exposure (dashboard unchanged)
- Not a replacement for tailnet access (LAN continues to work direct)
- No multi-user auth; single allowlisted email

## Deliverables

- `cloudflared` installed on box, authenticated, running as systemd unit
- Named tunnel `hangar` pointing to `localhost:8080`
- DNS record `optiplex.<domain>.com` → tunnel
- Cloudflare Access policy: email = `george.nijo8@gmail.com`, session = 30d
- `systemd/cloudflared-hangar.service` committed to repo
- `docs/phases/01-tunnel.md` updated with chosen domain

## Acceptance criteria

- From a device **not on the tailnet** (e.g. phone on cellular): `https://optiplex.<domain>.com/` triggers Cloudflare Access login, succeeds with George's Google account, and loads the Phase 0 dashboard
- Iframes (`/s/codex/`, `/s/wave/`, `/s/issue12/`) work through the tunnel
- `systemctl status cloudflared-hangar` is `active (running)` and restarts on box reboot
- Attempt to access from a non-allowlisted email returns 403
- TLS is valid (Cloudflare-issued cert)
- Latency over tunnel < 500 ms to first byte from cellular

## Dependencies

- A domain George controls, under Cloudflare management
- Cloudflare account with Access enabled (free tier is sufficient)
- Phase 0 shipped (this phase proxies to it)

## Risks / unknowns

- **Domain not decided** — blocking step, George picks a name
- **WebSocket passthrough** — Cloudflare Access sometimes interferes with long-lived ws; ttyd traffic may need session cookie tweaks. Test plan: open a tab on cellular and ensure terminal bytes flow for 5 min without disconnect.
- **Cold start** — first access after tunnel restart can be slow; set `no_tls_verify: false` but `connect_timeout` short.

## Estimated effort

~1 Claude-session of work. Straightforward apart from domain selection.

## Rollout

1. Pick domain
2. Run `cloudflared tunnel login` (interactive, George does this)
3. `cloudflared tunnel create hangar`
4. Commit the tunnel credentials file path (but not the file itself; `.gitignore` covers it)
5. Write `systemd/cloudflared-hangar.service` and `/etc/cloudflared/config.yml`
6. Create DNS record via `cloudflared tunnel route dns hangar optiplex.<domain>.com`
7. Create Cloudflare Access application with email policy
8. Enable and start service
9. Verify from phone cellular

## Rollback

`systemctl disable --now cloudflared-hangar`. DNS record removed. Tailnet access unaffected.
