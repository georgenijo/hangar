# hangar — Roadmap

One-page map of where we're going and when. Detail lives in [`PHASES.md`](PHASES.md) and the per-phase docs.

---

## Guiding goal

Build the **command center** George wants: one dashboard that shows every agent running anywhere, lets him click in, prompt, and get pinged when something needs attention. Visibility is the killer feature.

---

## Milestone map

```
NOW ─── tailnet-only stopgap dashboard (Phase 0) ✅

           │
           ▼
       PHASE 1   Public URL + SSO  (Cloudflare Tunnel + Access)
           │     → phone works from anywhere
           │
           ▼
       PHASE 2   MVP Command Center  (Rust + SvelteKit)
           │     → own PTYs, smart Claude Code UI, push, REST API
           │     → this is the "daily driver" milestone
           │
           ▼
       PHASE 3   Logs firehose in-dashboard
           │
           ▼
       PHASE 4   Deeper session intelligence + Codex driver
           │
           ▼
       PHASE 5   Recording + cross-session search
           │
           ▼
       PHASE 6   Sandboxing (podman + overlayfs)
           │
           ▼
       PHASE 7   Branching + snapshots ("save-state" for conversations)
           │
           ▼
       PHASE 8   Hooks + policy engine (user-defined rules)
           │
           ▼
       PHASE 9   Inter-agent protocols (sessions call each other)
           │
           ▼
       PHASE 10  Multi-node scheduler (laptop + box + cloud)
           │
           ▼
       PHASE 11  Voice + mobile-first UX
```

---

## What "shipped" means per phase

Each phase has its own `docs/phases/<NN>-*.md` with explicit acceptance criteria. In one line:

- **0** · Dashboard loads over tailnet, shows 3 tmux sessions as iframes, reload buttons work
- **1** · `optiplex.georgenijo.com` reaches the dashboard over HTTPS with SSO; phone loads it
- **2** · New backend runs; ttyd retired; SvelteKit UI shows Claude sessions as chat; can prompt over REST; push alerts on `awaiting`
- **3** · `/logs` page streams journald + unit + app logs with filters
- **4** · Codex driver matches Claude; dashboard shows model/tokens/idle-time per tile
- **5** · `/api/search?q=` across all session history; replay a session by scrubbing timeline
- **6** · Each new session runs in podman with overlayfs; fs-diff preview before merge
- **7** · `POST /api/sessions/:id/fork` works; branching UI shows parent/child tree
- **8** · Lua/Starlark scripts run on `before_prompt` / `after_response` / `on_tool_use`
- **9** · One session programmatically invokes another; job tree visible in dashboard
- **10** · Register laptop as a node; spawn session on chosen node; live-migrate snapshot
- **11** · PWA on phone with push-to-talk, STT, TTS; widget shows live session count

---

## What's intentionally not on this roadmap (and why)

- **Team/multi-user** — this is a personal tool; adding accounts would pull in scope (RBAC, billing, etc.) for no current benefit
- **Auto-update** — manual deploy for now; issue filed to revisit once phases 2-4 land
- **Global unique slugs** — spike item; deferred until multi-node demands it
- **Fork/parent field on Session** — not useful until branching phase; add then
- **Prometheus exporter** — plain JSON metrics cover our needs until we install Grafana
- **Open source** — stays private until at least after Phase 2; decide based on whether it generalizes

---

## Decision principles (in priority order)

1. **Visibility first** — if a phase doesn't make the dashboard more useful, defer it
2. **Keep the MVP small enough to ship** — resist bundling phases 3-5 into 2
3. **Every deferred feature gets an issue** — so nothing quietly disappears
4. **Document choices as ADRs** — future Claude sessions should be able to understand "why" without asking
5. **Tmux keeps running** during the whole build — the stopgap dashboard is kept working until hangar's own UI fully replaces it
6. **One artifact, many hosts** — hangar ships as a Linux container image. The Mac mini under OrbStack is the daily host; any cloud Linux VM is a drop-in alternative. See [ADR-0017](decisions/0017-containerize-deployment.md).

