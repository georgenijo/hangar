# Phase 6 — Sandboxing

**Status:** ⬜ Planned

## Goal

Every new session runs inside a container with its own overlay filesystem, network policy, and resource caps. Review fs diffs before merging to real tree.

## Non-goals

- Not replacing tools users actually invoke (podman stays on host; backend orchestrates)
- Not building a general-purpose container platform — just enough for hangar sessions

## Deliverables

- `sandbox` module: creates podman containers per session with:
  - overlayfs mount over host paths (read-only base, writable overlay)
  - cgroup v2 limits (CPU, memory, tokens-cost budget)
  - nftables egress rules (allowlisted hosts per session)
- New `SessionConfig.sandbox: Option<SandboxSpec>`
- UI: sandbox-status badge, fs-diff viewer pre-merge
- "Merge overlay" action: atomically apply overlay writes to real tree with backup snapshot

## Acceptance criteria

- New Claude session spawns inside podman
- Writes made by Claude show in overlay, not host, until merged
- Diff view shows added/modified/deleted files
- Network policy: Claude in `whoop` session can reach `github.com` but not `example.com` by default (configurable)
- CPU/memory caps enforced (verified by stress test)
- Merge applies overlay and creates a restic snapshot

## Dependencies

- Phase 2 shipped
- podman + overlayfs + cgroup v2 + nftables installed on box

## Risks / unknowns

- Performance overhead (expected < 5 %)
- Claude tools that expect host paths may break — mount strategy needs care
- GPU/accelerator passthrough (not needed now, may matter later)

## Estimated effort

4–5 Claude-sessions of work. Biggest phase after MVP.
