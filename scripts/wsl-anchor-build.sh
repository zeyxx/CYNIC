#!/bin/bash
#
# CYNIC Anchor Build (WSL)
#
# Quick build script for Anchor programs
#
# Usage:
#   wsl -d Ubuntu -- bash /mnt/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC/scripts/wsl-anchor-build.sh
#
# "φ distrusts φ" - κυνικός

set -e

# Setup paths
export PATH="$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$HOME/.avm/bin:$PATH"

cd /mnt/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC/programs/cynic-anchor

echo "🐕 Building CYNIC Anchor program..."
echo ""

# Use system cargo with v1.52 platform-tools for Rust 1.89 compatibility
RUSTUP_TOOLCHAIN=stable cargo build-sbf --tools-version v1.52

echo ""
echo "✓ Build complete!"
ls -la ../../target/deploy/*.so 2>/dev/null || echo "  (no .so files)"
