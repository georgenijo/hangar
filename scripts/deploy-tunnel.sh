#!/usr/bin/env bash
# Deploy Phase 1: cloudflared-hangar systemd unit.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
echo "repo: $REPO"

# 1. Install systemd unit
sudo install -m 644 "$REPO/systemd/cloudflared-hangar.service" /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now cloudflared-hangar

# 2. Status
echo "--- status ---"
systemctl is-active cloudflared-hangar
cloudflared tunnel info hangar 2>/dev/null || true

echo
echo "tunnel: https://optiplex.georgenijo.com/"
