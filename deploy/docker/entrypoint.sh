#!/usr/bin/env bash
# Entrypoint for the hangar container.
# Starts the supervisor (holds PTY fds), then Caddy (serves SPA + proxies API),
# then runs hangard in the foreground. tini (PID 1) reaps zombies and forwards
# signals.
set -euo pipefail

STATE_DIR="${HANGAR_STATE_DIR:-/state}"
mkdir -p "$STATE_DIR/sessions"

echo "[entrypoint] starting hangar-supervisor"
/usr/local/bin/hangar-supervisor &
SUPERVISOR_PID=$!

# Give supervisor a moment to create its Unix socket
for i in 1 2 3 4 5 6 7 8 9 10; do
	if [ -S "$STATE_DIR/hangar/supervisor.sock" ] || [ -S "$STATE_DIR/supervisor.sock" ]; then
		break
	fi
	sleep 0.2
done

echo "[entrypoint] starting caddy"
caddy run --config /etc/caddy/Caddyfile --adapter caddyfile &
CADDY_PID=$!

cleanup() {
	echo "[entrypoint] shutting down"
	kill -TERM "$CADDY_PID" 2>/dev/null || true
	kill -TERM "$SUPERVISOR_PID" 2>/dev/null || true
	wait 2>/dev/null || true
}
trap cleanup TERM INT

echo "[entrypoint] starting hangard"
exec /usr/local/bin/hangard
