#!/bin/bash
set -euo pipefail
source "$HOME/.cargo/env"

BINARY_DIR="$HOME/bin"
BINARY="$BINARY_DIR/cynic-kernel"
UNIT_DIR="$HOME/.config/systemd/user"
UNIT="$UNIT_DIR/cynic-kernel.service"
ENV_FILE="$HOME/.config/cynic/env"

# Skip deploy if no Rust files changed
CHANGED_FILES=$(git diff --name-only HEAD~1 2>/dev/null || echo "unknown")
RUST_CHANGED=$(echo "$CHANGED_FILES" | grep -E '\.(rs|toml)$' || true)

if [ -z "$RUST_CHANGED" ] && [ "$CHANGED_FILES" != "unknown" ]; then
    echo "[deploy] No .rs/.toml changes — skipping build+deploy"
    exit 0
fi

echo "[deploy] Building release..."
cargo build --release -p cynic-kernel

# Skip redeploy if binary hasn't changed
NEW_HASH=$(sha256sum target/release/cynic-kernel | cut -d' ' -f1)
OLD_HASH=$(sha256sum "$BINARY" 2>/dev/null | cut -d' ' -f1 || echo "none")

if [ "$NEW_HASH" = "$OLD_HASH" ]; then
    echo "[deploy] Binary unchanged (sha256 match) — skipping restart"
    exit 0
fi

echo "[deploy] Stopping service..."
systemctl --user stop cynic-kernel 2>/dev/null || true
sleep 1

echo "[deploy] Installing binaries (atomic)..."
mkdir -p "$BINARY_DIR"
cp target/release/cynic-kernel "$BINARY.new"
chmod +x "$BINARY.new"
mv -f "$BINARY.new" "$BINARY"
# MCP server is the same binary — keep in sync
cp "$BINARY" "$BINARY_DIR/cynic-mcp"

echo "[deploy] Installing environment..."
source "$HOME/.cynic-env" 2>/dev/null || { echo "[deploy] ERROR: ~/.cynic-env not found — create it with SURREALDB_PASS, CYNIC_API_KEY, CYNIC_REST_ADDR"; exit 1; }
mkdir -p "$(dirname "$ENV_FILE")"
cat > "$ENV_FILE" <<EOF
SURREALDB_URL=ws://localhost:8000
SURREALDB_USER=root
SURREALDB_PASS=${SURREALDB_PASS:?Set SURREALDB_PASS in ~/.cynic-env}
CYNIC_API_KEY=${CYNIC_API_KEY:-}
CYNIC_REST_ADDR=${CYNIC_REST_ADDR:-127.0.0.1:3030}
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
