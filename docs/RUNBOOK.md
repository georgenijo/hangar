# hangar — Runbook

How to operate the autonomous build and the running system.

---

## Running hangar locally (Mac mini, OrbStack)

```bash
# from the repo root
./scripts/deploy.sh container build   # build image
./scripts/deploy.sh container up      # start supervisor + hangard + caddy
open http://localhost:8080            # dashboard

./scripts/deploy.sh container logs    # tail logs
./scripts/deploy.sh container down    # stop
```

State (SQLite + ring files) persists in the named Docker volume `hangar-state`
across rebuilds. To wipe state: `docker volume rm hangar-state`.

## Running hangar on a cloud Linux VM

Same image, same compose file. On the VM:

```bash
git clone https://github.com/georgenijo/hangar.git
cd hangar
docker compose -f deploy/docker/compose.yml up -d
```

Cloudflare Tunnel terminates in front of the published `:8080` (run
`cloudflared` on the VM host, or add a sidecar container).

---

## Starting the autonomous build

### Prerequisites
- `~/Documents/code/hangar` cloned and on `main`
- `claude` CLI authenticated (`claude /login`)
- `gh` CLI authenticated
- At least one tmux session named `hangar-build` for visibility

### Kick-off

```bash
cd ~/Documents/code/hangar
git pull

# Start a tmux session so the build survives terminal disconnect
tmux new-session -d -s hangar-build

# Dispatch the pipeline in build order:
tmux send-keys -t hangar-build \
  './.pipeline/batch.sh --project-dir "$PWD" --issues 1,6,7,8,9,2,3,10,4' Enter
```

Watch via:
- `tmux attach -t hangar-build` to see live output
- `~/Documents/pipeline-logs/hangar/issue-<N>/` for per-issue artifacts
- The hangar dashboard at `http://localhost:8080` for live session view
- GitHub notifications for PRs/issue comments

### Parallel dispatch (faster)

Once the serial pipeline proves out, use `parallel.sh` to run multiple issues
concurrently — each in its own git worktree + tmux session. Same total token
cost, ~3× faster wall-clock on the Mac mini.

```bash
cd ~/Documents/code/hangar
./.pipeline/parallel.sh --project-dir "$PWD" --issues 6,7,8,9 --concurrent 3
```

Sessions are named `hangar-pipe-<issue>`. Attach any one with
`tmux attach -t hangar-pipe-6`. Worktrees live at
`~/Documents/code/hangar.worktrees/issue-<N>/`.

Do **not** parallelize issues with a strong ordering dependency (e.g. run Phase 1
alone before Phase 2.1 because 2.7 wants the tunnel); batch unrelated milestones
together. Rule of thumb: Phase 2.1–2.4 can all run in parallel; merge conflicts
are rare because they touch different modules.

### After Phase 2 ships (issue #4 closed)

Phase 3–6 were filed without the `ready` label. To unblock them:

```bash
cd ~/Documents/code/hangar
for n in 5 11 12 13; do
  gh issue edit "$n" --add-label ready
done

tmux send-keys -t hangar-build \
  './.pipeline/batch.sh --project-dir "$PWD" --issues 5,11,12,13' Enter
```

---

## Stopping / resuming the build

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
cd ~/Documents/code/hangar
gh pr create --fill --base main --head "$BRANCH"
gh pr merge --auto --squash
```

The PR waits for CI to go green, then squash-merges. If CI fails, PR stays open until a human fixes.

---

## Watching the running product

- Local dashboard: `http://localhost:8080/`
- Public dashboard (when CF Tunnel is repointed): TBD — see [ADR-0017](decisions/0017-containerize-deployment.md)
- Push notifications: subscribe on phone to the ntfy topic in `~/.config/hangar/config.toml`
- Logs: `./scripts/deploy.sh container logs` (or `docker compose -f deploy/docker/compose.yml logs -f`)
- Metrics: `curl http://localhost:8080/api/v1/metrics`

---

## Health checks

```bash
docker compose -f deploy/docker/compose.yml ps
docker compose -f deploy/docker/compose.yml exec hangar ps auxf
docker volume inspect hangar-state
```

---

## Backup

The state volume is the only thing worth backing up:
- `hangar-state` Docker volume (SQLite + ring files + supervisor socket)
- `~/.config/hangar/` on the host if used (push rules, config)
- The repo itself lives on GitHub

Mac mini: include `~/Library/Containers/dev.kdrag0n.OrbStackHelper/Data/data/docker/volumes/hangar-state/` in Time Machine, or run `docker run --rm -v hangar-state:/data -v $PWD:/backup alpine tar czf /backup/hangar-state.tgz -C /data .` on a schedule.

Cloud VM: include the Docker volume directory (`/var/lib/docker/volumes/hangar-state/`) in your provider's snapshot.
