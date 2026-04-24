#!/bin/bash
# CYNIC Kernel Deploy — build, verify, swap, smoke test, rollback on failure.
# Usage: ./scripts/deploy-kernel.sh
# R23-exempt: standalone script, sets own env

set -euo pipefail

KERNEL_BIN="${HOME}/bin/cynic-kernel"
KERNEL_BAK="${HOME}/bin/cynic-kernel.bak"
KERNEL_SRC="$(cd "$(dirname "$0")/.." && pwd)"
RELEASE_BIN="${KERNEL_SRC}/target/release/cynic-kernel"
SERVICE="cynic-kernel.service"

# Load env for health check
[ -f "$HOME/.cynic-env" ] && . "$HOME/.cynic-env"
HEALTH_URL="http://${CYNIC_REST_ADDR:-127.0.0.1:3030}/health"

echo "=== CYNIC Kernel Deploy ==="

# 1. Build release
echo "[1/5] Building release..."
export RUST_MIN_STACK=67108864
export RUSTFLAGS="-C debuginfo=1"
(cd "$KERNEL_SRC" && cargo build --release --quiet)
echo "  Built: $(ls -lh "$RELEASE_BIN" | awk '{print $5}')"

# 2. Hash check — don't deploy identical binary
CURRENT_HASH=$(md5sum "$KERNEL_BIN" 2>/dev/null | cut -d' ' -f1 || echo "none")
NEW_HASH=$(md5sum "$RELEASE_BIN" | cut -d' ' -f1)
if [ "$CURRENT_HASH" = "$NEW_HASH" ]; then
    echo "[2/5] Binary unchanged (hash: ${NEW_HASH:0:12}). Nothing to deploy."
    exit 0
fi
echo "[2/5] New binary (${CURRENT_HASH:0:12} → ${NEW_HASH:0:12})"

# 3. Swap binary (mv+cp pattern — MCP may hold the old inode)
echo "[3/5] Swapping binary..."
mv "$KERNEL_BIN" "$KERNEL_BAK"
cp "$RELEASE_BIN" "$KERNEL_BIN"

# 4. Restart service
echo "[4/5] Restarting $SERVICE..."
systemctl --user restart "$SERVICE"
sleep 5

# 5. Smoke test
echo "[5/5] Smoke test..."
STATUS=$(curl -s --max-time 5 "$HEALTH_URL" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('status','FAIL'))" 2>/dev/null || echo "FAIL")

if [ "$STATUS" = "sovereign" ] || [ "$STATUS" = "degraded" ]; then
    VERSION=$(curl -s --max-time 5 "$HEALTH_URL" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('version','?'))" 2>/dev/null || echo "?")
    echo "  PASS: status=$STATUS version=$VERSION"
    echo "=== Deploy complete ==="
else
    echo "  FAIL: status=$STATUS — rolling back"
    mv "$KERNEL_BAK" "$KERNEL_BIN"
    systemctl --user restart "$SERVICE"
    sleep 3
    ROLLBACK_STATUS=$(curl -s --max-time 5 "$HEALTH_URL" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('status','FAIL'))" 2>/dev/null || echo "FAIL")
    echo "  Rollback: status=$ROLLBACK_STATUS"
    exit 1
fi
