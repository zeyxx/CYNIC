#!/usr/bin/env bash
set -euo pipefail

# Source local coordination env without committing secrets or real addresses.
source ~/.cynic-env >/dev/null 2>&1 || true

if [[ -z "${CYNIC_REST_ADDR:-}" ]]; then
    echo "CYNIC_REST_ADDR not set. Source ~/.cynic-env before launching Gemini." >&2
    exit 1
fi

if [[ -z "${CYNIC_API_KEY:-}" ]]; then
    echo "CYNIC_API_KEY not set. Coordination tools beyond /health will fail." >&2
fi

exec ./mcp-coord/cynic-coord
