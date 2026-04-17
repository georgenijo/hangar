# hangar — Runbook

How to operate the autonomous build and the running system.

---

## Starting the autonomous build (on the box)

### Prerequisites
- Box reachable over Tailscale (`ssh george@optiplex` works)
- `~/Documents/hangar` cloned and on `main`
- `claude` CLI authenticated (`claude /login`)
- `gh` CLI authenticated
- At least one tmux session named `hangar-build` for visibility

### Kick-off

```bash
# on the box
cd ~/Documents/hangar
git pull

# Start a tmux session so the build survives SSH disconnect
tmux new-session -d -s hangar-build

# Dispatch the pipeline in build order:
# Phase 1 → Phase 2.1..2.4 → Phase 2.5..2.6 → Phase 2.7 → Phase 2.8
tmux send-keys -t hangar-build \
  './.pipeline/batch.sh --project-dir "$PWD" --issues 1,6,7,8,9,2,3,10,4' Enter
```

Watch via:
- `tmux attach -t hangar-build` to see live output
- `~/Documents/pipeline-logs/hangar/issue-<N>/` for per-issue artifacts
- The Phase 0 dashboard (`http://optiplex:8080`) for the tmux session containing Claude agents
- GitHub notifications for PRs/issue comments

### Parallel dispatch (faster)

Once the serial pipeline proves out, use `parallel.sh` to run multiple issues
concurrently — each in its own git worktree + tmux session. Same total token
cost, ~3× faster wall-clock on this box.

```bash
cd ~/Documents/hangar
./.pipeline/parallel.sh --project-dir "$PWD" --issues 6,7,8,9 --concurrent 3
```

Sessions are named `hangar-pipe-<issue>`. Attach any one with
`tmux attach -t hangar-pipe-6`. Worktrees live at
`~/Documents/hangar.worktrees/issue-<N>/`.

Do **not** parallelize issues with a strong ordering dependency (e.g. run Phase 1
alone before Phase 2.1 because 2.7 wants the tunnel); batch unrelated milestones
together. Rule of thumb: Phase 2.1–2.4 can all run in parallel; merge conflicts
are rare because they touch different modules.

### After Phase 2 ships (issue #4 closed)

Phase 3–6 were filed without the `ready` label. To unblock them:

```bash
cd ~/Documents/hangar
for n in 5 11 12 13; do
  gh issue edit "$n" --add-label ready
done

tmux send-keys -t hangar-build \
  './.pipeline/batch.sh --project-dir "$PWD" --issues 5,11,12,13' Enter
```

---

## Stopping / resuming

```bash
# stop
tmux kill-session -t hangar-build

# resume — the pipeline logs record completed steps and skip them on re-run
tmux new-session -d -s hangar-build
tmux send-keys -t hangar-build \
  './.pipeline/batch.sh --project-dir "$PWD" --issues <comma-list>' Enter
```

---

## Blocker handling

The pipeline is configured to **skip on failure** (per policy). When an issue fails:

1. Check `~/Documents/pipeline-logs/hangar/issue-<N>/` for the last completed step and any error
2. Post a comment on the issue describing what blocked it
3. Label the issue `blocked`
4. Fix the blocker (or leave for a human)
5. Remove `blocked`, add `ready`, and re-dispatch

Examples of likely blockers:
- `cloudflared tunnel login` needs interactive browser auth (Phase 1 issue #1)
- Domain not chosen for Phase 1
- Rust toolchain missing on box (install `rustup default stable`)
- `claude` CLI not logged in

---

## Auto-merge policy

Merge policy: **auto-merge if CI green**. The pipeline creates a branch and commits; a GitHub Action or manual `gh pr create && gh pr merge --auto` step finalizes. For the MVP we rely on:

```bash
# From the builder agent's shell (pipeline provides this as the last step):
cd ~/Documents/hangar
gh pr create --fill --base main --head "$BRANCH"
gh pr merge --auto --squash
```

The PR waits for CI to go green, then squash-merges. If CI fails, PR stays open until a human fixes.

---

## Watching the running product

After Phase 2 ships:

- Public dashboard: `https://optiplex.<domain>/` (Phase 1 tunnel)
- Tailnet dashboard: `http://optiplex:8080/`
- Push notifications: subscribe on phone to the ntfy topic in `~/.config/hangar/config.toml`
- Logs: `journalctl -u hangar -f`
- Metrics: `curl http://optiplex:8080/api/v1/metrics`

---

## Health checks

```bash
systemctl status hangar hangar-supervisor caddy cloudflared-hangar
ss -tlnp | grep -E ':(3000|8080)'
du -sh ~/.local/state/hangar/
```

---

## Backup

`restic` already runs on the box. Ensure it covers:
- `~/.local/state/hangar/` (SQLite + ring files)
- `~/.config/hangar/` (push rules, config)
- `~/Documents/hangar/` (repo, if not relying on GitHub)
