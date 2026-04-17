# ADR-0002: tmux stays as stopgap, not the substrate

**Status:** Accepted
**Date:** 2026-04-17
**Phase:** 0 — 2

## Context

tmux is the default multiplexer for terminal sessions on the box. We could (a) build hangar on top of tmux (treat tmux as the durable session layer forever), or (b) use tmux only for the Phase 0 stopgap dashboard and have hangar own its own PTYs from Phase 2 onward.

## Options considered

1. **tmux as permanent substrate.** Backend talks to tmux via `tmux -CC` control mode. Cheap to start, no PTY code. Free durability via `tmux-resurrect`.
2. **tmux as stopgap, then own the PTYs.** ttyd-per-session dashboard today. From Phase 2, hangar spawns and owns PTYs directly with `portable-pty`. tmux continues to exist for George's ad-hoc SSH use but isn't in hangar's path.
3. **Own the PTYs from day one.** Skip the stopgap. Delays any visible progress by weeks.

## Decision

Option 2. Phase 0 dashboard uses ttyd + tmux (ships immediately). Phase 2 introduces the Rust backend, which owns PTYs. tmux stays installed and used interactively outside hangar; hangar does not invoke tmux.

## Consequences

- Good: instant visible value (Phase 0 shipped in a session)
- Good: hangar can introduce structured session types, typed events, and snapshotting without fighting tmux's model
- Good: tmux-resurrect and muscle memory preserved for ad-hoc work
- Bad: brief dual-stack period during Phase 2 build (hangar + ttyd running side-by-side)
- Bad: Rust backend has to reinvent PTY reattach/durability logic (covered by ADR-0010)

## Future work

- Decommission ttyd + Phase 0 iframes once hangar replaces them (scheduled at end of Phase 2 migration)
