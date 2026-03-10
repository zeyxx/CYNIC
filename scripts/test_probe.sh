#!/bin/bash
# Delete cached node.toml to force a fresh first-boot probe
rm -f ~/.cynic/node.toml

. ~/.cargo/env
export CARGO_TARGET_DIR=/tmp/cynic-build

echo "=== Booting CYNIC Ring 0 Probe Test ==="
timeout 20 /tmp/cynic-build/debug/cynic-kernel
echo ""
echo "=== ~/.cynic/node.toml ==="
cat ~/.cynic/node.toml 2>/dev/null || echo "[FAIL] node.toml not created"
