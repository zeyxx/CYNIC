#!/bin/bash
set -euo pipefail
source "$HOME/.cargo/env"

BINARY_DIR="$HOME/bin"
BINARY="$BINARY_DIR/cynic-kernel"
UNIT_DIR="$HOME/.config/systemd/user"
UNIT="$UNIT_DIR/cynic-kernel.service"
SURREAL_PASS=$(cat ~/.surreal-pass)

echo "[deploy] Building release..."
cargo build --release -p cynic-kernel

echo "[deploy] Installing binary..."
mkdir -p "$BINARY_DIR"
cp target/release/cynic-kernel "$BINARY"
chmod +x "$BINARY"

echo "[deploy] Installing systemd unit..."
mkdir -p "$UNIT_DIR"
cat > "$UNIT" <<EOF
[Unit]
Description=CYNIC OS V2 — Sovereign Kernel
After=network.target

[Service]
ExecStart=$BINARY
Restart=on-failure
RestartSec=5
Environment=SURREALDB_URL=ws://localhost:8000
Environment=SURREALDB_USER=root
Environment=SURREALDB_PASS=$SURREAL_PASS

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable cynic-kernel
systemctl --user restart cynic-kernel
sleep 1
systemctl --user status cynic-kernel --no-pager | head -12

echo "[deploy] ✅ cynic-kernel deployed and running"
