# ADR-0017: Containerize deployment, OrbStack on host

**Status:** Accepted
**Date:** 2026-05-12
**Phase:** Cross-cutting (replaces optiplex/systemd host model from ADR-0006 / ADR-0010 era)

## Context

Hangar was originally designed around a single dedicated host (Dell OptiPlex 7050 running Ubuntu 24.04, ADR-0013) with systemd-managed services and a static install path. In practice the optiplex is underpowered for the workload and the user wants:

1. **More compute** — multiple Claude/Codex sessions in parallel without thermal throttling. The new host is a Mac mini M4 / 24 GB.
2. **Compartmentalization** — agents should not have unrestricted run of the host filesystem and processes.
3. **Portability** — the same deployable should drop onto a cloud Linux VM (AWS EC2, Oracle Cloud free tier, Fly, Hetzner) without a port.

A straight macOS native build is possible (gate sandbox module behind `cfg(target_os = "linux")`, write launchd plists — issues #88 and #89) but yields a mac-only artifact and abandons the existing Linux-only sandboxing design (Phase 6, podman + overlayfs). It also leaves the cloud VM story unresolved.

## Options considered

1. **Native macOS build, mac-only host** — close #88 and #89, run hangard under launchd directly on the mac. Trade-offs: simplest dev loop, but Phase 6 sandboxing has to be redesigned for macOS, no cloud portability without a second build target, agents share the host filesystem.

2. **Linux container on the mac via OrbStack, same image on cloud VMs** — keep the Rust target Linux-only. Bind-mount host code and credentials into the container. Trade-offs: one more layer to learn, ~5–10 second container restart on the mac, but the existing Linux-only sandbox design survives, image is portable, agents are fenced from the host filesystem by container boundaries.

3. **Full VM on the mac (UTM, multipass)** — Linux native inside the VM. Trade-offs: heavier than a container, no real upside over OrbStack for a single-app deployment.

## Decision

Hangar's canonical deployment artifact is a **multi-arch Linux container image** built from `deploy/docker/Dockerfile`. Locally it runs under OrbStack on `george-mac-mini` (arm64). For cloud VMs the same image runs under Docker or Podman (amd64 or arm64).

The image bundles:
- `hangard` (backend daemon)
- `hangar-supervisor` (PTY supervisor)
- frontend SPA (static files served by Caddy inside the container)
- Caddy (reverse proxy on `:8080`)
- `node` + `npm` runtime (so agent CLIs like `@anthropic-ai/claude-code` can be installed at runtime or layered in)

The compose file (`deploy/docker/compose.yml`) defines:
- Port `:8080` published to host
- Named volume `hangar-state` mounted at `/state` — SQLite DB, ring buffers, supervisor socket survive image rebuilds
- Bind mounts: host `~/Documents/code` → `/code` (sessions land here, agents edit your code in place), and a named volume `hangar-home` at `/home/hangar` for OAuth tokens, claude/codex config, project history.

Native macOS build (#88 / #89) is **deprioritized**, not won't-fix. It can still be useful for a future "no container, just run a binary" mode, but it is not on the critical path.

## Consequences

- **Good:**
  - One artifact targets local mac mini and any cloud Linux VM.
  - Phase 6 sandboxing design (podman + overlayfs) survives — we're inside Linux semantics.
  - Container boundary keeps agents from touching host files except where explicitly mounted.
  - `docker compose up -d` is the install path everywhere; no host-specific service files.
  - `~/.local/state/hangar` paths in the existing code map cleanly to a `/state` volume — backend already honors `HANGAR_STATE_DIR`.

- **Bad:**
  - Adds a runtime dependency on OrbStack (mac) or Docker (cloud). For a single-user tool this is acceptable.
  - First boot includes a multi-minute Rust release build. Future work: cargo-chef layer caching.
  - Cloudflare Tunnel needs to reach the published port — either run `cloudflared` on the mac host pointing at `:8080`, or add a sidecar `cloudflared` container.
  - The "supervisor keeps PTYs alive while backend restarts" pattern (ADR-0010) only protects within-container restarts. If the whole container is killed, sessions reset.

- **Future work opened up:**
  - Multi-arch buildx pipeline (arm64 + amd64) for cloud VM deploys.
  - Cargo-chef caching to drop subsequent build time to seconds.
  - Decide whether Phase 6 nested-container sandboxing is still worth building or whether outer-container isolation is sufficient.
  - `cloudflared` sidecar pattern for the public URL.
  - Backup story replacement for the existing `restic` job on the optiplex (Time Machine on the mac, or a backup sidecar).

## Supersedes / relates to

- Updates the host assumption in ADR-0013 (tailnet-only auth) — the boundary now sits in front of the published container port, not in front of the optiplex.
- Reopens the design space referenced in ADR-0006 (no sandboxing in MVP) and the Phase 6 phase doc.
- Does **not** change ADR-0003 (Rust + axum + portable-pty), ADR-0004 (SQLite + ring buffer), or ADR-0010 (supervisor pattern) — they all work inside the container.
