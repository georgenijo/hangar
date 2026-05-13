#!/usr/bin/env bash
# Usage: deploy.sh <container|backend|phase0|help> [...]
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE_FILE="$REPO/deploy/docker/compose.yml"
PROJECT_NAME="hangar"

case "${1:-}" in
container)
	sub="${2:-up}"
	case "$sub" in
	build)
		echo "=== building container image ==="
		docker compose -f "$COMPOSE_FILE" -p "$PROJECT_NAME" build
		;;
	up)
		echo "=== starting hangar ==="
		docker compose -f "$COMPOSE_FILE" -p "$PROJECT_NAME" up -d
		echo
		docker compose -f "$COMPOSE_FILE" -p "$PROJECT_NAME" ps
		echo
		echo "dashboard: http://localhost:8080"
		;;
	down)
		echo "=== stopping hangar ==="
		docker compose -f "$COMPOSE_FILE" -p "$PROJECT_NAME" down
		;;
	logs)
		docker compose -f "$COMPOSE_FILE" -p "$PROJECT_NAME" logs -f --tail=200
		;;
	restart)
		docker compose -f "$COMPOSE_FILE" -p "$PROJECT_NAME" restart
		;;
	status|ps)
		docker compose -f "$COMPOSE_FILE" -p "$PROJECT_NAME" ps
		;;
	shell)
		docker compose -f "$COMPOSE_FILE" -p "$PROJECT_NAME" exec hangar bash
		;;
	rebuild)
		echo "=== rebuilding from scratch ==="
		docker compose -f "$COMPOSE_FILE" -p "$PROJECT_NAME" down
		docker compose -f "$COMPOSE_FILE" -p "$PROJECT_NAME" build --no-cache
		docker compose -f "$COMPOSE_FILE" -p "$PROJECT_NAME" up -d
		;;
	*)
		echo "Usage: deploy.sh container <build|up|down|logs|restart|status|shell|rebuild>" >&2
		exit 1
		;;
	esac
	;;

backend)
	# Legacy path: build natively and restart systemd services on a Linux host.
	# Used for cloud-VM deployments that don't use the container image. See
	# docs/decisions/0017-containerize-deployment.md for the canonical flow.
	echo "=== pulling latest ==="
	git -C "$REPO" pull

	echo "=== building backend ==="
	(cd "$REPO/backend" && cargo build --release)

	echo "=== building frontend ==="
	(cd "$REPO/frontend" && pnpm install --frozen-lockfile && pnpm build)

	echo "=== restarting hangar ==="
	sudo systemctl restart hangar

	echo "=== reloading caddy ==="
	sudo systemctl reload caddy || sudo systemctl restart caddy

	echo "--- status ---"
	systemctl is-active hangar caddy
	;;

phase0)
	# Emergency Phase 0 rollback. The ttyd .service files were removed from the
	# repo in Phase 2.8. To use this subcommand you must first restore them:
	#   git checkout <phase0-commit> -- systemd/ttyd-codex.service systemd/ttyd-wave.service systemd/ttyd-issue12.service
	# Then re-run this script.
	echo "WARNING: installing Phase 0 stopgap"

	pkill -f "ttyd -p 7682" || true
	pkill -f "ttyd -p 7683" || true
	pkill -f "ttyd -p 7684" || true

	for s in ttyd-codex ttyd-wave ttyd-issue12; do
		sudo install -m 644 "$REPO/systemd/$s.service" /etc/systemd/system/
	done
	sudo systemctl daemon-reload
	for s in ttyd-codex ttyd-wave ttyd-issue12; do
		sudo systemctl enable --now "$s"
	done

	sudo install -m 644 "$REPO/caddy/Caddyfile.phase0" /etc/caddy/Caddyfile
	sudo mkdir -p /var/log/caddy
	sudo chown caddy:caddy /var/log/caddy 2>/dev/null || true
	sudo systemctl enable --now caddy
	sudo systemctl reload caddy || sudo systemctl restart caddy

	echo "--- status ---"
	systemctl is-active ttyd-codex ttyd-wave ttyd-issue12 caddy
	ss -tlnp 2>/dev/null | grep -E ':(7682|7683|7684|8080)' || true
	;;

help|"")
	cat <<'EOF'
Usage: deploy.sh <subcommand>

Primary (local + cloud-VM with Docker):
  container build        build the image
  container up           start hangar (detached) and print status
  container down         stop hangar
  container logs         tail compose logs
  container restart      restart services
  container status       compose ps
  container shell        bash into the hangar container
  container rebuild      down + no-cache build + up

Legacy (cloud-VM with systemd, not the canonical path):
  backend                git pull + cargo build + pnpm build + systemctl restart

Emergency:
  phase0                 install Phase 0 ttyd stopgap (requires restored service files)
EOF
	;;

*)
	echo "Error: unknown subcommand '$1'" >&2
	echo "Run 'deploy.sh help' for usage" >&2
	exit 1
	;;
esac
