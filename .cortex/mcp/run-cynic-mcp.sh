#!/usr/bin/env bash
set -euo pipefail

runtime_env="${CYNIC_RUNTIME_ENV_FILE:-/home/user/.config/cynic/env}"
mcp_binary="${CYNIC_MCP_BINARY:-/home/user/bin/cynic-kernel-mcp}"

if [[ ! -r "${runtime_env}" ]]; then
  echo "cynic MCP blocked: runtime environment is unavailable" >&2
  exit 1
fi

if [[ ! -x "${mcp_binary}" ]]; then
  echo "cynic MCP blocked: executable is unavailable" >&2
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "${runtime_env}"
set +a

: "${CYNIC_SOVEREIGN_URL:?cynic MCP blocked: CYNIC_SOVEREIGN_URL is missing}"
: "${CYNIC_REST_ADDR:?cynic MCP blocked: CYNIC_REST_ADDR is missing}"
: "${CYNIC_API_KEY:?cynic MCP blocked: CYNIC_API_KEY is missing}"

# The MCP boundary requires the sovereign kernel identity. The local REST
# address remains useful to host-local adapters, but is not coordination SSOT.
export CYNIC_REST_ADDR="${CYNIC_SOVEREIGN_URL}"

exec "${mcp_binary}"
