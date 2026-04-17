#!/usr/bin/env bash
# Usage: deploy.sh <backend|phase0|help>
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
echo "repo: $REPO"

case "${1:-}" in
backend)
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
	echo "dashboard: http://optiplex:8080"
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

	echo
	echo "dashboard: http://optiplex:8080"
	;;

help)
	echo "Usage: deploy.sh <subcommand>"
	echo ""
	echo "Subcommands:"
	echo "  backend   git pull + cargo build + pnpm build + systemctl restart hangar"
	echo "  phase0    Emergency rollback: install Phase 0 ttyd stopgap (requires service files restored from git history)"
	echo "  help      Show this message"
	exit 0
	;;

*)
	echo "Error: subcommand required" >&2
	echo "Run 'deploy.sh help' for usage" >&2
	exit 1
	;;
esac
