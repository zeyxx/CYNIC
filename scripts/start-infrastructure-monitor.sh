#!/bin/bash
# Wrapper script for infrastructure monitor with environment loading
# Sources ~/.cynic-env and runs the infrastructure consumer

set -e

# Get project root (use CYNIC_ROOT from systemd, fallback to git)
PROJECT_ROOT="${CYNIC_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null)}"
if [ -z "$PROJECT_ROOT" ]; then
    echo "ERROR: CYNIC_ROOT not set and git rev-parse failed" >&2
    exit 1
fi

# Source environment variables
source "$HOME/.cynic-env"

# Get kernel URL from SSOT endpoint (no implicit localhost fallback)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/cynic-endpoint.sh"
KERNEL_URL="$(cynic_kernel_rest_url)"

# Run the infrastructure monitor (polls infrastructure observations every 60s)
exec /usr/bin/python3 "$PROJECT_ROOT/cynic-python/consumers/k15_infrastructure_consumer.py" \
  --kernel-url "$KERNEL_URL" \
  --api-key "$CYNIC_API_KEY" \
  --poll-interval 60
