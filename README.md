# hangar

Web control panel for tmux sessions + system health on the Optiplex box.

## Phases

- **0** — ttyd per session + Caddy reverse proxy + static HTML dashboard (tailnet-only)
- **1** — Cloudflare tunnel + Access (public URL with auth)
- **2** — Rust backend (axum + tmux -CC) replaces per-session ttyds
- **3** — Session intelligence (pane state detection, badges)
- **4** — Phone push (ntfy/APNs on state transitions)
- **5** — Logs firehose (journal + units + panes + apps → unified stream)
- **6** — REST automation + iOS Shortcuts
- **7** — Recording + cross-session search

## Phase 0 layout

```
caddy/Caddyfile          reverse proxy :8080 → ttyd ports, static HTML
systemd/ttyd-*.service   one unit per tmux session
web/index.html           dashboard with tabs + grid + reload buttons
scripts/deploy.sh        install units, reload caddy
```

## Deploy (on the box)

```
cd ~/Documents/hangar
./scripts/deploy.sh
```

Open http://optiplex:8080 from any tailnet device.

## Add a new tmux session

1. `tmux new-session -d -s <name>` (or link-window from grouped session)
2. Copy `systemd/ttyd-codex.service` → `systemd/ttyd-<name>.service`, bump port, change session name
3. Edit `web/index.html` — add `<name>` to `SESSIONS` array
4. Edit `caddy/Caddyfile` — add `handle_path /s/<name>*` block
5. `./scripts/deploy.sh`

Future phases automate this via the Rust backend.
