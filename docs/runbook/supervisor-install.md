# Supervisor install runbook

How to install and run `hangar-supervisor` as a systemd user unit so PTY
sessions survive a `hangard` restart.

Related:
- Parent: [#8 — Phase 2.3 Supervisor](https://github.com/georgenijo/hangar/issues/8)
- ADR: [`docs/decisions/0010-sessions-survive-restart.md`](../decisions/0010-sessions-survive-restart.md)
- Unit files: [`systemd/hangar-supervisor.service`](../../systemd/hangar-supervisor.service), [`systemd/hangar.service`](../../systemd/hangar.service)

---

## Architecture (one paragraph)

`hangar-supervisor` is a small long-lived daemon that owns every PTY fd.
`hangard` (the Rust backend) connects to it over a Unix socket and receives
PTY fds via `SCM_RIGHTS`. When `hangard` restarts, the supervisor keeps
running, child processes stay alive under it (via `PR_SET_CHILD_SUBREAPER`),
and the new `hangard` re-attaches every fd on startup. If the supervisor
itself dies, children are reparented to PID 1 and become unrecoverable —
which is why the unit uses `Restart=always` and `KillMode=process` (a
supervisor stop must NOT cascade-kill its children).

Socket: `~/.local/state/hangar/supervisor.sock` (derived from
`dirs::state_dir()` — resolves to `$XDG_STATE_HOME/hangar/` or
`~/.local/state/hangar/`).

---

## Prerequisites

- Linux with systemd user instance (`systemd --user`) available.
- Repo checked out at `~/Documents/hangar` (if different, replace paths below).
- Release binaries built: `backend/target/release/hangar-supervisor` and
  `backend/target/release/hangard`.
  ```bash
  cd ~/Documents/hangar/backend
  cargo build --release --bin hangar-supervisor --bin hangard
  ```
- A live graphical / lingering user session so the user systemd instance
  is running. Headless boxes need `loginctl enable-linger $(whoami)` or
  the units will stop when you log out.

---

## Install steps (dev box — supervisor only)

This is the mode currently used on the optiplex dev box: `hangard` runs
manually in a tmux pane via `cargo run --release --bin hangard`, so the
`hangar.service` unit is installed but **not** enabled (enabling it would
race the manual hangard for port 3000).

```bash
cd ~/Documents/hangar

# 1. Install unit files into the user systemd search path.
install -d ~/.config/systemd/user
install -m 644 systemd/hangar-supervisor.service ~/.config/systemd/user/
install -m 644 systemd/hangar.service           ~/.config/systemd/user/

# 2. Pick up the new unit files.
systemctl --user daemon-reload

# 3. Enable + start the supervisor. Safe to re-run (idempotent).
systemctl --user enable --now hangar-supervisor.service

# 4. Verify.
systemctl --user is-enabled hangar-supervisor   # -> enabled
systemctl --user is-active  hangar-supervisor   # -> active
ls -l ~/.local/state/hangar/supervisor.sock     # -> srw-rw---- ...
```

Then (re)start `hangard` in your dev tmux pane and confirm the handshake:

```bash
curl -s http://localhost:3000/api/v1/health | jq .
# {"status":"ok","supervisor_connected":true,"uptime_s":...}
```

The backend logs should also show:

```
INFO  hangard: connected to supervisor at "/home/$USER/.local/state/hangar/supervisor.sock"
```

## Install steps (production — both units managed)

Same as above but also enable `hangar.service`. `hangar.service` has
`Requires=hangar-supervisor.service` + `After=hangar-supervisor.service`, so
starting hangar will start the supervisor first.

```bash
systemctl --user enable --now hangar-supervisor.service
systemctl --user enable --now hangar.service

systemctl --user status hangar --no-pager | head
curl -s http://localhost:3000/api/v1/health | jq .supervisor_connected
# true
```

Do **not** do this on the optiplex dev box while a manual `cargo run`
hangard is holding port 3000 — the unit will fail with `AddrInUse`.

---

## Uninstall / rollback

```bash
systemctl --user disable --now hangar hangar-supervisor 2>/dev/null || true
rm -f ~/.config/systemd/user/hangar.service \
      ~/.config/systemd/user/hangar-supervisor.service
systemctl --user daemon-reload
rm -f ~/.local/state/hangar/supervisor.sock
```

## Upgrading after a binary rebuild

Because the unit uses `ExecStart=%h/Documents/hangar/backend/target/release/hangar-supervisor`,
rebuilding the binary does **not** update the running supervisor — you
must restart it. After `cargo build --release` finishes:

```bash
systemctl --user restart hangar-supervisor
```

Note: restarting the supervisor during active PTY work is safe only if the
new supervisor starts fast enough to re-adopt subreaper'd children before
they exit. For dev, prefer to drain sessions first.

---

## Verification (#36 survive-restart smoke test)

Manual smoke test that a live session survives a `hangard` restart.
Uses a `shell` session (no Claude auth required).

```bash
# 0. Sanity.
curl -s http://localhost:3000/api/v1/health | jq .supervisor_connected  # -> true

# 1. Create a shell session.
SID=$(curl -sS -X POST http://localhost:3000/api/v1/sessions \
  -H 'content-type: application/json' \
  -d '{"slug":"supervisor-smoke","kind":{"type":"shell"},"project_dir":"/home/george/Documents/hangar"}' \
  | jq -r .session_id)
echo "session=$SID"

# 2. Write something identifiable into the PTY.
curl -sS -X POST "http://localhost:3000/api/v1/sessions/$SID/prompt" \
  -H 'content-type: application/json' \
  -d '{"prompt":"echo HANGAR_SMOKE_$(date +%s) > /tmp/hangar-smoke.txt\n"}' >/dev/null

# 3. Record pre-restart state.
PID_BEFORE=$(pgrep -f 'target/release/hangard' | head -1)
STATE_BEFORE=$(curl -s "http://localhost:3000/api/v1/sessions/$SID" | jq -r .state)
echo "before: hangard_pid=$PID_BEFORE state=$STATE_BEFORE"

# 4. Restart hangard. In dev (manual cargo run) send SIGTERM and restart it
#    in its tmux pane; in prod use: systemctl --user restart hangar
kill -TERM "$PID_BEFORE"
# ...wait for the dev pane to relaunch hangard, or `systemctl --user restart hangar`
sleep 5

# 5. Post-restart state.
PID_AFTER=$(pgrep -f 'target/release/hangard' | head -1)
STATE_AFTER=$(curl -s "http://localhost:3000/api/v1/sessions/$SID" | jq -r .state)
CONNECTED=$(curl -s http://localhost:3000/api/v1/health | jq -r .supervisor_connected)
echo "after:  hangard_pid=$PID_AFTER state=$STATE_AFTER supervisor_connected=$CONNECTED"

# 6. Assertions.
test "$PID_AFTER"    != "$PID_BEFORE"    && echo "OK: hangard pid changed"
test "$STATE_AFTER"  =  "running"        && echo "OK: session still running"
test "$CONNECTED"    =  "true"           && echo "OK: supervisor reconnected"

# 7. Prove the PTY still accepts writes.
curl -sS -X POST "http://localhost:3000/api/v1/sessions/$SID/prompt" \
  -H 'content-type: application/json' \
  -d '{"prompt":"cat /tmp/hangar-smoke.txt\n"}' >/dev/null
```

Expected result: `hangard_pid` differs before/after, `state=running` both
before and after, `supervisor_connected=true` after.

If `state` flips to `exited` after restart, do **not** fix in-place —
file a detailed bug on #38 with: the two state payloads, `hangard` logs
spanning the restart, and `journalctl --user -u hangar-supervisor` for
the same window.

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `supervisor_connected: false` in health | `hangar-supervisor` not running, or socket path mismatch | `systemctl --user status hangar-supervisor`; confirm socket at `~/.local/state/hangar/supervisor.sock` |
| `Failed to connect to bus: No medium found` | `XDG_RUNTIME_DIR` unset in this shell | `export XDG_RUNTIME_DIR=/run/user/$(id -u); export DBUS_SESSION_BUS_ADDRESS=unix:path=$XDG_RUNTIME_DIR/bus` |
| Unit stops when you log out | no user lingering | `loginctl enable-linger $(whoami)` |
| `hangar.service` fails with `Address already in use` | a manual `cargo run hangard` is holding port 3000 | stop the manual one, or keep `hangar.service` disabled on dev boxes |
| Sessions flip to `exited` after restart | supervisor reparenting or fd-passing regression | see #36/#38 — capture logs, do not retry blindly |
