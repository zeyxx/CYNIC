#!/bin/bash
# ts_introspect_call.sh — Call ts_introspect via JSON-RPC on tailscale-mcp subprocess.
# Usage: ts_introspect_call.sh <node> <service> [port]
# Returns JSON: {running, failure_reason, port_bound, process_id, service_state, ...}

set -euo pipefail

NODE="${1:?node required}"
SERVICE="${2:?service required}"
PORT="${3:-0}"  # 0 = auto-lookup from ServiceRegistry

# MCP server config (stdio-based, spawned on-demand)
# Fallback: derive from git root if TAILSCALE_MCP not set
_GIT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo ".")
MCP_PROG="${TAILSCALE_MCP:-${_GIT_ROOT}/../tailscale-mcp/tailscale-mcp}"

if [[ ! -f "$MCP_PROG" ]]; then
    echo '{"error":"MCP binary not found at '"$MCP_PROG"'"}' >&2
    exit 1
fi

# JSON-RPC 2.0 call to ts_introspect
# See: tailscale-mcp/mcp/introspect.go introspectHandler
read -r -d '' CALL << 'EOF' || true
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "ts_introspect",
    "arguments": {
      "node": "%s",
      "service": "%s",
      "port": %d
    }
  },
  "id": 1
}
EOF

REQUEST=$(printf "$CALL" "$NODE" "$SERVICE" "$PORT")

# Spawn MCP, send request, capture response
RESPONSE=$("$MCP_PROG" <<< "$REQUEST" 2>/dev/null || echo '{"error":"MCP call failed"}')

# Extract result text (MCP returns {jsonrpc, result:{content:[{type,text}]}, id})
# Parse the nested JSON from result.content[0].text
echo "$RESPONSE" | jq -r '.result.content[0].text // .error // "null"' 2>/dev/null || echo '{"error":"JSON parse failed"}'
