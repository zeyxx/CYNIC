#!/bin/bash
# Wrapper script for infrastructure monitor with environment loading
# Sources ~/.cynic-env and runs the infrastructure consumer

set -e

# Get project root (use CYNIC_ROOT from systemd, fallback to git)
PROJECT_ROOT="${CYNIC_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || echo /home/user/Bureau/CYNIC)}"
if [ -z "$PROJECT_ROOT" ]; then
    echo "ERROR: CYNIC_ROOT not set and all fallbacks failed" >&2
    exit 1
fi

# Source environment variables
source "$HOME/.cynic-env"

# Get kernel URL from env (CYNIC_REST_ADDR should be set in ~/.cynic-env)
KERNEL_URL="${CYNIC_REST_ADDR}"
if [ -z "$KERNEL_URL" ]; then
    echo "ERROR: CYNIC_REST_ADDR not set in ~/.cynic-env" >&2
    exit 1
fi

# Convert host:port to http://host:port if needed
if [[ ! "$KERNEL_URL" =~ ^http ]]; then
    KERNEL_URL="http://$KERNEL_URL"
fi

# Run the infrastructure monitor (polls infrastructure observations every 60s)
exec /usr/bin/python3 "$PROJECT_ROOT/cynic-python/consumers/k15_infrastructure_consumer.py" \
  --kernel-url "$KERNEL_URL" \
  --api-key "$CYNIC_API_KEY" \
  --poll-interval 60
