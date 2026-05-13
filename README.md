# hangar

**Personal agent operating system.** A self-hosted control plane for AI coding agents (Claude Code, Codex, and others), shipped as a Linux container that runs locally under OrbStack on a Mac and identically on any cloud Linux VM. Think `tmux + kubectl + OBS` reimagined for someone who lives inside multiple Claude sessions.

Status: v0.2.0 shipped Phase 2 MVP (Rust backend, SvelteKit dashboard, ntfy push, REST prompt API). Now migrating from the original optiplex host to a containerized deploy on the Mac mini ([ADR-0017](docs/decisions/0017-containerize-deployment.md)).

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
- Anyone who has an always-on box (Mac mini, home server, cloud VM) and a laptop/phone they hop between
- People who want to build on top of a typed session/agent model rather than scrape ANSI terminal bytes

Not for: shared team installs, multi-tenant SaaS, or people who are happy with a single terminal window.

---

## What's here today (v0.2.0, containerizing in progress)

```
./backend/                  Rust backend (hangard + hangar-supervisor)
./frontend/                 SvelteKit SPA dashboard
./caddy/Caddyfile           Reverse proxy :8080 → backend + static dashboard
./deploy/docker/            Dockerfile + compose.yml for the containerized deploy
./scripts/deploy.sh         container build/up/down (local) and backend (legacy systemd)
```

Run it locally:

```bash
./scripts/deploy.sh container build
./scripts/deploy.sh container up
open http://localhost:8080
```

Same compose file runs on any cloud Linux VM with Docker.

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

Primary host is `george-mac-mini` running OrbStack. See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md#environment) for the deployment model. Backend is Rust (`hangard`, `hangar-supervisor`), frontend is SvelteKit, both built into a single Linux container image.

Local dev workflow:

```
edit → commit → push to github.com/georgenijo/hangar
./scripts/deploy.sh container build
./scripts/deploy.sh container up
```

Cloud VM workflow: same `Dockerfile`, deploy via `docker compose up -d`. See [`docs/decisions/0017-containerize-deployment.md`](docs/decisions/0017-containerize-deployment.md) for rationale. Specifics land in each phase doc.

---

## License

Private repo, no license granted. To be decided before any public release.
