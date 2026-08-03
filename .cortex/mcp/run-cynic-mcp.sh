#!/usr/bin/env bash
set -euo pipefail

runtime_root="${CYNIC_RUNTIME_INSTALL_ROOT:-/var/lib/cynic/runtime}"
runtime_mcp="$runtime_root/current/cynic-mcp"
mcp_binary="${CYNIC_MCP_BINARY:-}"

if [[ -z "${mcp_binary}" && -x "${runtime_mcp}" ]]; then
  mcp_binary="${runtime_mcp}"
fi

if [[ -z "${mcp_binary}" ]]; then
  mcp_binary="cynic-mcp"
fi

if [[ "${mcp_binary}" != */* ]]; then
  mcp_binary="$(command -v "${mcp_binary}" || true)"
fi

if [[ -z "${mcp_binary}" || ! -x "${mcp_binary}" ]]; then
  echo "cynic MCP blocked: executable is unavailable" >&2
  exit 1
fi

if ! timeout 5 "${mcp_binary}" --contract-json >/dev/null; then
  echo "cynic MCP blocked: running artifact cannot prove the required contract revision and capabilities" >&2
  exit 1
fi

export CYNIC_AGENT_TYPE="${CYNIC_AGENT_TYPE:-codex}"

exec "${mcp_binary}"
