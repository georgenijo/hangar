# hangar — Phases

Index for the phased build. Each phase has its own detailed doc in [`phases/`](phases).

Phases are ordered by dependency, not by calendar. A phase does not start until its predecessor has shipped its acceptance criteria and been used for at least a week of real work.

---

## Phase status legend

- ✅ **Shipped** — criteria met, in production
- 🟡 **In progress** — work has started
- ⬜ **Planned** — documented, not started
- 💭 **Aspirational** — documented as a direction, specifics TBD

---

## Phases

| # | Name | Status | Doc |
|---|---|---|---|
| 0 | ttyd stopgap dashboard | ✅ | [00-ttyd-stopgap.md](phases/00-ttyd-stopgap.md) |
| 1 | Cloudflare Tunnel + Access | ⬜ | [01-tunnel.md](phases/01-tunnel.md) |
| 2 | MVP Command Center (Rust + SvelteKit) | ⬜ | [02-mvp-command-center.md](phases/02-mvp-command-center.md) |
| 3 | Logs firehose | ⬜ | [03-logs-firehose.md](phases/03-logs-firehose.md) |
| 4 | Deeper intelligence + Codex driver | ⬜ | [04-intelligence.md](phases/04-intelligence.md) |
| 5 | Recording + search | ⬜ | [05-recording-search.md](phases/05-recording-search.md) |
| 6 | Sandboxing | ⬜ | [06-sandboxing.md](phases/06-sandboxing.md) |
| 7 | Branching + snapshots | 💭 | [07-branching.md](phases/07-branching.md) |
| 8 | Hooks + policy engine | 💭 | [08-hooks-policy.md](phases/08-hooks-policy.md) |
| 9 | Inter-agent protocols | 💭 | [09-inter-agent.md](phases/09-inter-agent.md) |
| 10 | Multi-node scheduler | 💭 | [10-multi-node.md](phases/10-multi-node.md) |
| 11 | Voice + mobile UX | 💭 | [11-voice-mobile.md](phases/11-voice-mobile.md) |

---

## How each phase doc is structured

Every `phases/<NN>-*.md` should contain:

- **Goal** — what success looks like in one paragraph
- **Non-goals** — what this phase deliberately isn't
- **Deliverables** — files/components produced
- **Acceptance criteria** — the bar for calling it shipped (objective, testable)
- **Dependencies** — prior phases, external tools, credentials needed
- **Risks / unknowns** — things that could derail
- **Estimated effort** — rough size (Claude-sessions of work, not calendar)
- **Rollout plan** — how we deploy without breaking previous phases
- **Rollback plan** — how we back out if needed

Aspirational phases may omit `Rollout` / `Rollback` until they move to `Planned`.

---

## MVP definition

The MVP — "I use this instead of tmux every day" — is Phase 2. It deliberately bundles what would be three smaller phases (REST control, Claude Code UI, push notifications) into a single release because none of them are individually worth shipping without the others.

Phases 3+ are strictly feature growth beyond the MVP.
