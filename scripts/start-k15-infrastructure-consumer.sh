#!/bin/bash
# Wrapper script for K15 infrastructure consumer with environment loading
# Sources ~/.cynic-env and runs the infrastructure consumer
# Routes probe failures to recovery decisions

set -e

# Get project root (R1: no hardcoded paths)
PROJECT_ROOT="${CYNIC_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null)}"
if [ -z "$PROJECT_ROOT" ]; then
    echo "ERROR: CYNIC_ROOT not set and git rev-parse failed" >&2
    exit 1
fi

# Source environment variables (use /root/.cynic-env since this runs as root in systemd)
source /root/.cynic-env

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

# Run the infrastructure consumer (polls /observations, routes failures to /inference/remediate)
# CYNIC_API_KEY from environment, not CLI args — prevents ps aux leakage
exec /usr/bin/python3 "$PROJECT_ROOT/cynic-python/consumers/k15_infrastructure_consumer.py" \
  --kernel-url "$KERNEL_URL" \
  --poll-interval 60
