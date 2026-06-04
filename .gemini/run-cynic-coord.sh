#!/usr/bin/env bash
set -euo pipefail

# Source local coordination env without committing secrets or real addresses.
source ~/.cynic-env >/dev/null 2>&1 || true

# Dynamic discovery: Look up Kernel IP from registry if not in env
if [[ -z "${CYNIC_REST_ADDR:-}" ]]; then
    # Parse registry.json for the proxmox_host_kernel node
    KERNEL_IP=$(jq -r '.nodes[] | select(.role == "proxmox_host_kernel") | .network.tailscale' infra/registry.json)
    
    if [[ -n "$KERNEL_IP" && "$KERNEL_IP" != "null" ]]; then
        export CYNIC_REST_ADDR="http://$KERNEL_IP:3030"
        echo "Discovered Kernel at $CYNIC_REST_ADDR from registry." >&2
    fi
fi

if [[ -z "${CYNIC_REST_ADDR:-}" ]]; then
    echo "CYNIC_REST_ADDR not set and discovery failed. Source ~/.cynic-env or update infra/registry.json." >&2
    exit 1
fi

if [[ -z "${CYNIC_API_KEY:-}" ]]; then
    echo "CYNIC_API_KEY not set. Coordination tools beyond /health will fail." >&2
fi

exec ./mcp-coord/cynic-coord
