# hangar

**Personal agent operating system.** A self-hosted control plane for AI coding agents (Claude Code, Codex, and others), running on a dedicated always-on box. Think `tmux + kubectl + OBS` reimagined for someone who lives inside multiple Claude sessions.

Status: early. Phase 0 shipped (ttyd + caddy stopgap dashboard). Phase 1+ in planning. See [docs/ROADMAP.md](docs/ROADMAP.md).

---

## Why this exists

Running several Claude Code / Codex sessions in parallel on a remote box via tmux + SSH works but has rough edges:

- No visibility — you have to attach to know what each agent is doing
- No automation — prompting from phone / iOS Shortcut / cron means SSH gymnastics
- No notifications — Claude stalls waiting for a yes/no and you have no idea
- No memory — once a session is closed, its history is gone
- No intelligence — nothing parses what Claude is actually doing, it's all terminal bytes

hangar is the tool that sits above tmux (and later replaces it) to give you a real control plane: web UI, push notifications, REST API, session history, multi-agent orchestration.

**Primary goal:** a command-center dashboard where you can see every running agent at a glance, click in to any session, drive prompts from anywhere, and be alerted when any session needs attention.

---

## Who this is for

- Solo developers running multi-session AI-agent workflows
- Anyone who has a dedicated always-on box (home server, cloud VM) and a laptop/phone they hop between
- People who want to build on top of a typed session/agent model rather than scrape ANSI terminal bytes

Not for: shared team installs, multi-tenant SaaS, or people who are happy with a single terminal window.

---

## What's here today (Phase 0)

```
./caddy/Caddyfile           Reverse proxy :8080 → ttyd backends + static dashboard
./systemd/ttyd-*.service    One ttyd per tmux session (codex, wave, issue12)
./web/index.html            Static dashboard: tabs, grid view, reload buttons
./scripts/deploy.sh         Installs units, reloads caddy
```

Served at `http://optiplex:8080` on the tailnet. Per-session iframe at `/s/<name>/`.

---

## What's coming

1. **Phase 1** — Cloudflare Tunnel + Access → public URL with SSO, phone-ready
2. **Phase 2 (MVP)** — Rust backend replaces ttyd; SvelteKit UI; Claude Code smart parsing; phone push; REST prompt API
3. **Phase 3+** — Logs firehose, deeper intelligence, sandboxing, branching, hooks, multi-node, voice UX

See [`docs/PHASES.md`](docs/PHASES.md) for the full phased plan and [`docs/ROADMAP.md`](docs/ROADMAP.md) for the one-page view.

---

## Docs index (for humans and agents)

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — system design, boundaries, data model (read first)
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — one-page milestone map
- [`docs/PHASES.md`](docs/PHASES.md) — phase index with pointers to detailed per-phase docs
- [`docs/phases/`](docs/phases) — one file per phase (goals, deliverables, acceptance criteria, risks, estimates)
- [`docs/SESSION-PROTOCOL.md`](docs/SESSION-PROTOCOL.md) — session lifecycle, state machine, agent driver spec, wire format
- [`docs/decisions/`](docs/decisions) — ADRs (architectural decision records)
- [`CHANGELOG.md`](CHANGELOG.md) — release notes

Agents working in this repo: read `ARCHITECTURE.md` then the current phase doc in `docs/phases/` before writing code.

---

## Working in this repo

Primary environment is the Optiplex box (tailnet host `optiplex`). See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md#environment) for the deployment model. Backend code will be Rust; frontend is SvelteKit (incoming in Phase 2). Phase 0 is plain HTML.

Local dev workflow:

```
laptop: edit → commit → push to github.com/georgenijo/hangar
box:    git pull → cargo build --release → systemctl restart hangar
```

Specifics land in each phase doc.

---

## License

Private repo, no license granted. To be decided before any public release.
