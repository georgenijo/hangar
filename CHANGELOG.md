# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-04-17

### Added

- `hangard` Rust backend: Tokio/Axum HTTP server, session CRUD, PTY ownership via `portable-pty`, SQLite + 100 MB ring-buffer persistence, event broadcast bus
- `hangar-supervisor` binary: holds PTY fds across backend restarts via Unix socket handoff
- SvelteKit SPA dashboard: command-center tile view, session detail (chat UI for Claude Code, xterm.js for shell/raw), WebSocket live updates with exponential backoff
- ntfy push integration: rule engine triggers phone notification when Claude session transitions to `Awaiting`
- `POST /api/v1/sessions/:id/prompt` — send prompt to Claude session from anywhere
- `POST /api/v1/broadcast` — send text to N sessions
- `GET /api/v1/metrics` — JSON with `sessions_active`, `tokens_today`, `rss_mb`, `uptime_s`
- `systemd/hangar.service` and `systemd/hangar-supervisor.service` — supervised restart-on-failure
- `scripts/deploy.sh backend` subcommand: `git pull → cargo build → pnpm build → systemctl restart hangar`
- ADR `0016-self-host-ntfy-behind-caddy.md`

### Changed

- `caddy/Caddyfile`: routes `/api/*` and `/ws/*` to `hangard :3000`; SvelteKit SPA serves at `/`
- `scripts/deploy.sh` converted to subcommand dispatcher (`backend` / `phase0` / `help`)
- `docs/phases/02-mvp-command-center.md` marked ✅ Complete

### Removed

- Phase 0 ttyd stopgap: `ttyd-codex`, `ttyd-wave`, `ttyd-issue12` systemd units deleted from repo and disabled on box
- `/s/codex`, `/s/wave`, `/s/issue12` Caddy routes
- `/v2/*` Caddy route and `web/v2/` static Phase 2.1 interim UI (superseded by SvelteKit SPA)
- Ports 7682–7684 no longer in use

## [0.1.0]

### Added
- Phase 0: ttyd-based stopgap dashboard at `http://optiplex:8080`
  - Caddy reverse-proxy `/s/<name>` → ttyd ports (7682-7684)
  - Systemd units for `ttyd-codex`, `ttyd-wave`, `ttyd-issue12`
  - Static HTML dashboard with tabs, grid view, reload buttons
- Documentation set for the whole project:
  - `docs/ARCHITECTURE.md` — one-page system design
  - `docs/ROADMAP.md` — one-page milestone map
  - `docs/PHASES.md` — phase index
  - `docs/phases/00..11-*.md` — per-phase detailed docs
  - `docs/SESSION-PROTOCOL.md` — session lifecycle and agent driver spec
  - `docs/decisions/0001..0015-*.md` — ADRs covering major architectural choices
- `CHANGELOG.md` (this file)
- Phase 1: Cloudflare Tunnel + Access for public HTTPS
  - Named tunnel `hangar` → `https://optiplex.georgenijo.com/`
  - Cloudflare Access SSO gate (single-email allowlist)
  - Systemd unit `cloudflared-hangar.service`
  - Deploy script `scripts/deploy-tunnel.sh`

### Changed
- Renamed repo + working directory on the box from `optiplex-dashboard` to `hangar`

### Deferred / filed as issues
- Deployment strategy upgrades (auto-update, containers)
- Globally unique session slugs (multi-node naming)
- Forking / branching sessions
- Rich Codex driver (uses raw-bytes fallback until Phase 4)
- Sandboxing (Phase 6)

[Unreleased]: https://github.com/georgenijo/hangar/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/georgenijo/hangar/compare/e5ec0de...v0.2.0
