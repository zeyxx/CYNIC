#!/bin/bash
# Wrapper script for K15 consumer with environment loading
# Sources ~/.cynic-env and runs the consumer

set -e

# Source environment variables
source /home/user/.cynic-env

# Get kernel URL from env or use default (CYNIC_REST_ADDR should be set in ~/.cynic-env)
KERNEL_URL="${CYNIC_REST_ADDR}"
if [ -z "$KERNEL_URL" ]; then
    echo "ERROR: CYNIC_REST_ADDR not set in ~/.cynic-env" >&2
    exit 1
fi
# Convert host:port to http://host:port if needed
if [[ ! "$KERNEL_URL" =~ ^http ]]; then
    KERNEL_URL="http://$KERNEL_URL"
fi

# Run the consumer
exec /usr/bin/python3 /home/user/Bureau/CYNIC/cynic-python/consumers/k15_observation_consumer.py \
  --kernel-url "$KERNEL_URL" \
  --api-key "$CYNIC_API_KEY" \
  --domain twitter \
  --poll-interval 300
