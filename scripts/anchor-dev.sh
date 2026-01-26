#!/bin/bash
#
# CYNIC Anchor Development Container
#
# Builds and runs the Anchor dev environment in Docker
#
# Usage:
#   ./scripts/anchor-dev.sh          # Interactive shell
#   ./scripts/anchor-dev.sh build    # Build program
#   ./scripts/anchor-dev.sh deploy   # Deploy to devnet
#   ./scripts/anchor-dev.sh test     # Run Anchor tests
#
# "φ distrusts φ" - κυνικός

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
IMAGE_NAME="cynic-anchor-dev"

# Colors
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() {
    echo -e "${CYAN}🐕 $1${NC}"
}

# Build Docker image if needed
build_image() {
    if ! docker image inspect "$IMAGE_NAME" &> /dev/null; then
        log "Building Anchor dev image..."
        docker build -t "$IMAGE_NAME" -f "$PROJECT_DIR/docker/anchor-dev/Dockerfile" "$PROJECT_DIR"
    fi
}

# Run command in container
run_in_container() {
    docker run --rm -it \
        -v "$PROJECT_DIR:/workspace" \
        -v "$HOME/.config/solana:/root/.config/solana" \
        -e "HELIUS_API_KEY=${HELIUS_API_KEY:-}" \
        "$IMAGE_NAME" \
        "$@"
}

# Main
build_image

case "${1:-shell}" in
    build)
        log "Building Anchor program..."
        run_in_container anchor build
        log "Build complete! Artifacts in target/deploy/"
        ;;
    deploy)
        log "Deploying to devnet..."
        run_in_container bash -c '
            solana config set --url devnet
            if [ -f /root/.config/solana/id.json ]; then
                solana program deploy target/deploy/cynic_anchor.so
            else
                echo "No keypair found. Run: solana-keygen new"
            fi
        '
        ;;
    test)
        log "Running Anchor tests..."
        run_in_container anchor test
        ;;
    init)
        log "Initializing program on devnet..."
        run_in_container bash -c '
            solana config set --url devnet
            # Add initialization logic here
            echo "Program initialization - implement as needed"
        '
        ;;
    shell|*)
        log "Starting interactive shell..."
        echo -e "${YELLOW}Commands: anchor build, anchor test, solana ...${NC}"
        run_in_container /bin/bash
        ;;
esac
