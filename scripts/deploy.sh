#!/bin/bash
set -euo pipefail
source "$HOME/.cargo/env"

BINARY_DIR="$HOME/bin"
BINARY="$BINARY_DIR/cynic-kernel"
UNIT_DIR="$HOME/.config/systemd/user"
UNIT="$UNIT_DIR/cynic-kernel.service"
ENV_FILE="$HOME/.config/cynic/env"

echo "[deploy] Building release..."
cargo build --release -p cynic-kernel

echo "[deploy] Stopping service..."
systemctl --user stop cynic-kernel 2>/dev/null || true
sleep 1

echo "[deploy] Installing binary (atomic)..."
mkdir -p "$BINARY_DIR"
cp target/release/cynic-kernel "$BINARY.new"
chmod +x "$BINARY.new"
mv -f "$BINARY.new" "$BINARY"

echo "[deploy] Installing environment..."
mkdir -p "$(dirname "$ENV_FILE")"
cat > "$ENV_FILE" <<EOF
SURREALDB_URL=ws://localhost:8000
SURREALDB_USER=root
SURREALDB_PASS=$(cat ~/.surreal-pass)
EOF
chmod 600 "$ENV_FILE"

echo "[deploy] Installing systemd unit..."
mkdir -p "$UNIT_DIR"
cat > "$UNIT" <<EOF
[Unit]
Description=CYNIC OS V2 — Sovereign Kernel
After=network.target

[Service]
Type=simple
ExecStart=$BINARY
KillMode=mixed
KillSignal=SIGTERM
TimeoutStopSec=30
Restart=on-failure
RestartSec=3
EnvironmentFile=$ENV_FILE
LimitNOFILE=65535

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable cynic-kernel
systemctl --user start cynic-kernel

echo "[deploy] Waiting for startup..."
sleep 2
if systemctl --user is-active cynic-kernel >/dev/null 2>&1; then
    systemctl --user status cynic-kernel --no-pager | head -12
    echo "[deploy] cynic-kernel deployed and running"
else
    echo "[deploy] WARNING: service not active, checking logs..."
    journalctl --user -u cynic-kernel -n 10 --no-pager
    exit 1
fi
