#!/usr/bin/env bash
set -euo pipefail

runtime_env="${CYNIC_RUNTIME_ENV_FILE:-${XDG_CONFIG_HOME:-${HOME}/.config}/cynic/env}"
mcp_binary="${CYNIC_MCP_BINARY:-${HOME}/bin/cynic-kernel-mcp}"

if [[ ! -r "${runtime_env}" ]]; then
  echo "cynic MCP blocked: runtime environment is unavailable" >&2
  exit 1
fi

if [[ ! -x "${mcp_binary}" ]]; then
  echo "cynic MCP blocked: executable is unavailable" >&2
  exit 1
fi

if ! timeout 5 "${mcp_binary}" --contract-json >/dev/null; then
  echo "cynic MCP blocked: running artifact cannot prove the required contract revision and capabilities" >&2
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "${runtime_env}"
set +a

: "${CYNIC_API_KEY:?cynic MCP blocked: CYNIC_API_KEY is missing}"

# The MCP boundary requires the registry-owned kernel identity. Keep inference
# endpoints and stale host-local REST defaults out of coordination discovery.
export CYNIC_REST_ADDR="${CYNIC_COORD_KERNEL_URL:-http://cynic-kernel-primary.synapse.cynic:3030}"

exec "${mcp_binary}"
