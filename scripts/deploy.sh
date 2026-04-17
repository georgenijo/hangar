#!/usr/bin/env bash
# Deploy Phase 0: systemd units for ttyd + Caddy, reload services.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
echo "repo: $REPO"

# 1. Kill stray nohup ttyds from Phase -1
pkill -f "ttyd -p 7682" || true
pkill -f "ttyd -p 7683" || true
pkill -f "ttyd -p 7684" || true

# 2. Install systemd units
for s in ttyd-codex ttyd-wave ttyd-issue12; do
  sudo install -m 644 "$REPO/systemd/$s.service" /etc/systemd/system/
done
sudo systemctl daemon-reload
for s in ttyd-codex ttyd-wave ttyd-issue12; do
  sudo systemctl enable --now "$s"
done

# 3. Install Caddyfile
sudo install -m 644 "$REPO/caddy/Caddyfile" /etc/caddy/Caddyfile
sudo mkdir -p /var/log/caddy
sudo chown caddy:caddy /var/log/caddy 2>/dev/null || true
sudo systemctl enable --now caddy
sudo systemctl reload caddy || sudo systemctl restart caddy

# 4. Deploy backend (Phase 2)
if [[ -d "$REPO/backend" ]]; then
  echo "--- backend ---"
  (cd "$REPO/backend" && cargo build --release)
  install -m 755 "$REPO/backend/target/release/hangard" /home/george/.cargo/bin/hangard
  install -m 755 "$REPO/backend/target/release/hangar-supervisor" /home/george/.cargo/bin/hangar-supervisor
  for s in hangar-supervisor hangar; do
    sudo install -m 644 "$REPO/systemd/$s.service" /etc/systemd/system/
  done
  sudo systemctl daemon-reload
  sudo systemctl restart hangar-supervisor
  sudo systemctl restart hangar
  echo "--- backend status ---"
  systemctl is-active hangar-supervisor hangar || true
fi

# 5. Status
echo "--- status ---"
systemctl is-active ttyd-codex ttyd-wave ttyd-issue12 caddy
ss -tlnp 2>/dev/null | grep -E ':(7682|7683|7684|8080|3000)' || true

echo
echo "dashboard: http://optiplex:8080"
