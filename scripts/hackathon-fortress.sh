#!/bin/bash
# hackathon-fortress.sh — Make CYNIC survive any reboot/crash
# Run ONCE before the hackathon. After this, papa just needs to power on the PC.
set -euo pipefail

echo "=== CYNIC Hackathon Fortress Setup ==="

# ── 1. Install binary ──────────────────────────────
echo "[1/5] Building release binary..."
source "$HOME/.cargo/env"
cargo build --release -p cynic-kernel
mkdir -p "$HOME/bin"
cp target/release/cynic-kernel "$HOME/bin/cynic-kernel"
chmod +x "$HOME/bin/cynic-kernel"

# ── 2. Environment file (all secrets) ──────────────
echo "[2/5] Writing environment file..."
mkdir -p "$HOME/.config/cynic"
# Source existing env to get the values
source "$HOME/.cynic-env"
cat > "$HOME/.config/cynic/env" <<EOF
GEMINI_API_KEY=${GEMINI_API_KEY}
SURREALDB_URL=http://localhost:8000
SURREALDB_USER=root
SURREALDB_PASS=${SURREALDB_PASS}
CYNIC_REST_ADDR=0.0.0.0:3030
EOF
chmod 600 "$HOME/.config/cynic/env"

# ── 3. SurrealDB systemd service ──────────────────
echo "[3/5] Installing SurrealDB service..."
mkdir -p "$HOME/.config/systemd/user"
cat > "$HOME/.config/systemd/user/surrealdb.service" <<EOF
[Unit]
Description=SurrealDB — CYNIC storage
After=network.target

[Service]
Type=simple
ExecStart=$(which surreal) start --user root --pass ${SURREALDB_PASS} --bind 0.0.0.0:8000 memory
Restart=on-failure
RestartSec=3
KillMode=mixed
KillSignal=SIGTERM
TimeoutStopSec=10

[Install]
WantedBy=default.target
EOF

# ── 4. CYNIC kernel systemd service ──────────────
echo "[4/5] Installing CYNIC kernel service..."
cat > "$HOME/.config/systemd/user/cynic-kernel.service" <<EOF
[Unit]
Description=CYNIC OS V2 — Sovereign Kernel
After=network.target surrealdb.service
Requires=surrealdb.service

[Service]
Type=simple
WorkingDirectory=$HOME/Bureau/CYNIC
ExecStartPre=/bin/sleep 3
ExecStart=$HOME/bin/cynic-kernel
KillMode=mixed
KillSignal=SIGTERM
TimeoutStopSec=30
Restart=on-failure
RestartSec=5
EnvironmentFile=$HOME/.config/cynic/env
LimitNOFILE=65535

[Install]
WantedBy=default.target
EOF

# ── 5. Enable and start everything ────────────────
echo "[5/5] Enabling services..."
systemctl --user daemon-reload

# Kill manual processes first
pkill -f "surreal start" 2>/dev/null || true
pkill -f "cynic-kernel" 2>/dev/null || true
sleep 2

systemctl --user enable surrealdb
systemctl --user enable cynic-kernel
systemctl --user start surrealdb
sleep 3
systemctl --user start cynic-kernel
sleep 3

# Enable lingering so user services survive logout
loginctl enable-linger "$(whoami)" 2>/dev/null || echo "[WARN] enable-linger failed (may need sudo)"

echo ""
echo "=== Verification ==="
echo "SurrealDB: $(systemctl --user is-active surrealdb)"
echo "CYNIC:     $(systemctl --user is-active cynic-kernel)"
echo ""
curl -s http://localhost:3030/health | python3 -m json.tool 2>/dev/null || echo "[WARN] Health check failed, may need a moment..."
echo ""
echo "=== Fortress Ready ==="
echo "Services auto-restart on crash (RestartSec=3-5s)"
echo "Services auto-start on boot (enable-linger)"
echo "Papa instruction: just power on the PC and wait 30 seconds"
