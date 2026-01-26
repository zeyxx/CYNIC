#!/bin/bash
#
# CYNIC Anchor Setup for WSL
#
# Run this ONCE in WSL Ubuntu to install Solana + Anchor
# After that, just run: anchor build
#
# Usage (from Windows):
#   wsl -d Ubuntu
#   cd /mnt/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC
#   bash scripts/wsl-setup-anchor.sh
#
# "φ distrusts φ" - κυνικός

set -e

echo "🐕 CYNIC Anchor Setup for WSL"
echo "=============================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if already installed
if command -v solana &> /dev/null && command -v anchor &> /dev/null; then
    echo -e "${GREEN}✓ Solana and Anchor already installed!${NC}"
    solana --version
    anchor --version
    echo ""
    echo "You can now run: anchor build"
    exit 0
fi

echo "Installing dependencies..."
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libudev-dev \
    llvm \
    libclang-dev \
    protobuf-compiler \
    libssl-dev \
    curl

# Install Rust if not present
if ! command -v rustc &> /dev/null; then
    echo ""
    echo -e "${YELLOW}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo -e "${GREEN}✓ Rust already installed${NC}"
fi

# Ensure cargo is in path
source "$HOME/.cargo/env" 2>/dev/null || true

# Install Solana CLI
SOLANA_VERSION="2.1.15"
if ! command -v solana &> /dev/null; then
    echo ""
    echo -e "${YELLOW}Installing Solana CLI v${SOLANA_VERSION}...${NC}"
    sh -c "$(curl -sSfL https://release.anza.xyz/v${SOLANA_VERSION}/install)"
    export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
    echo 'export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"' >> ~/.bashrc
else
    echo -e "${GREEN}✓ Solana already installed${NC}"
fi

# Install Anchor via AVM
ANCHOR_VERSION="0.32.1"
if ! command -v anchor &> /dev/null; then
    echo ""
    echo -e "${YELLOW}Installing Anchor v${ANCHOR_VERSION} via AVM...${NC}"
    cargo install --git https://github.com/coral-xyz/anchor avm --force
    avm install ${ANCHOR_VERSION}
    avm use ${ANCHOR_VERSION}
else
    echo -e "${GREEN}✓ Anchor already installed${NC}"
fi

echo ""
echo "=============================="
echo -e "${GREEN}✓ Setup complete!${NC}"
echo ""
solana --version
anchor --version
echo ""
echo "Next steps:"
echo "  cd /mnt/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC"
echo "  anchor build"
echo ""
echo "🐕 *tail wag*"
