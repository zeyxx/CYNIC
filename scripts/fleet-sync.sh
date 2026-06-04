#!/usr/bin/env bash
# CYNIC Fleet Sync — Pushes updates to remote nodes
# Usage: ./scripts/fleet-sync.sh <node-name>
# Example: ./scripts/fleet-sync.sh cynic-forge

set -euo pipefail

NODE_NAME="${1:?Usage: $0 <node-name>}"
PROJECT_ROOT="$(git rev-parse --show-toplevel)"

source ~/.cynic-env 2>/dev/null || true

# 1. Resolve remote IP from fleet.toml
FLEET_TOML="${HOME}/.config/cynic/fleet.toml"
if [[ ! -f "$FLEET_TOML" ]]; then
    echo "Error: fleet.toml not found at $FLEET_TOML" >&2
    exit 1
fi

# Extract IP (assuming machine.<node-name>.tailscale_ip)
# Simple grep/sed for speed, replacing full TOML parser
REMOTE_IP=$(grep -A 10 "\[machine.${NODE_NAME}\]" "$FLEET_TOML" | grep "tailscale_ip" | cut -d'"' -f2)

if [[ -z "$REMOTE_IP" ]]; then
    echo "Error: Could not find tailscale_ip for node '$NODE_NAME' in fleet.toml" >&2
    exit 1
fi

echo "═══ Syncing to ${NODE_NAME} (${REMOTE_IP}) ═══"

# 2. Build binaries locally (if needed) or let remote build
# For simplicity and cross-platform safety, we'll let the remote pull and build.
# (Rule 11: OS consistency allows this)

# 3. Remote execution via SSH
ssh -o ConnectTimeout=5 "$REMOTE_IP" << EOF
    set -e
    echo "▶ Updating repository..."
    cd ~/dev/CYNIC || {
        mkdir -p ~/dev
        git clone https://github.com/zeyxx/CYNIC.git ~/dev/CYNIC
        cd ~/dev/CYNIC
    }
    git fetch origin
    git reset --hard origin/main

    echo "▶ Synchronizing environment..."
    mkdir -p ~/.config/cynic
    # Local .cynic-env is sensitive, usually managed out-of-band or via secure vault.
    # Here we assume it exists or is managed via Tailscale-locked transfer.

    echo "▶ Building cynic-node..."
    cargo build -p cynic-node --release

    echo "▶ Installing systemd units..."
    ./scripts/deploy-systemd.sh
EOF

echo "✓ Sync complete for ${NODE_NAME}"
