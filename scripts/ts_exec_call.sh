#!/bin/bash
# ts_exec_call.sh — Call ts_exec via JSON-RPC on tailscale-mcp subprocess.
# Usage: ts_exec_call.sh <node> <command> [timeout_secs]
# Returns JSON: {stdout, stderr, exit_code} or {error}

set -euo pipefail

NODE="${1:?node required}"
COMMAND="${2:?command required}"
TIMEOUT="${3:-30}"  # Default 30 seconds

# MCP server config (stdio-based, spawned on-demand)
# R23-exempt: subprocess needs explicit path; env var preferred, HOME fallback
MCP_PROG="${TAILSCALE_MCP:-${HOME}/Bureau/tailscale-mcp/tailscale-mcp}"

if [[ ! -f "$MCP_PROG" ]]; then
    echo '{"error":"MCP binary not found at '"$MCP_PROG"'"}' >&2
    exit 1
fi

# JSON-RPC 2.0 call to ts_exec
# See: tailscale-mcp/mcp/exec.go execHandler
read -r -d '' CALL << 'EOF' || true
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "ts_exec",
    "arguments": {
      "node": "%s",
      "command": "%s",
      "timeout": %d
    }
  },
  "id": 1
}
EOF

REQUEST=$(printf "$CALL" "$NODE" "$COMMAND" "$TIMEOUT")

# Spawn MCP, send request, capture response
RESPONSE=$("$MCP_PROG" <<< "$REQUEST" 2>/dev/null || echo '{"error":"MCP call failed"}')

# Extract result text (MCP returns {jsonrpc, result:{content:[{type,text}]}, id})
# Parse the nested JSON from result.content[0].text
echo "$RESPONSE" | jq -r '.result.content[0].text // .error // "null"' 2>/dev/null || echo '{"error":"JSON parse failed"}'
