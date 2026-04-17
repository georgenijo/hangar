# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/georgenijo/hangar/compare/e5ec0de...HEAD
