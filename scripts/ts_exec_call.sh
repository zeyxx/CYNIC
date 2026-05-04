#!/bin/bash
# ts_exec_call.sh — Call ts_exec via JSON-RPC on tailscale-mcp subprocess.
# Usage: ts_exec_call.sh <node> <command> [timeout_secs]
# Returns JSON: {stdout, stderr, exit_code} or {error}

set -euo pipefail

NODE="${1:?node required}"
COMMAND="${2:?command required}"
TIMEOUT="${3:-30}"  # Default 30 seconds

# MCP server config (stdio-based, spawned on-demand)
# Use env override or git-root-relative path as fallback
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo ".")"
MCP_PROG="${TAILSCALE_MCP:-$REPO_ROOT/../tailscale-mcp/tailscale-mcp}"

if [[ ! -f "$MCP_PROG" ]]; then
    echo '{"error":"MCP binary not found at '"$MCP_PROG"'"}' >&2
    exit 1
fi

# JSON-RPC 2.0 call to ts_exec (single line to avoid MCP parse errors)
# See: tailscale-mcp/mcp/exec.go execHandler
REQUEST=$(printf '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"ts_exec","arguments":{"node":"%s","command":"%s","timeout":%d}},"id":1}' "$NODE" "$COMMAND" "$TIMEOUT")

# Spawn MCP, send request, capture response
RESPONSE=$("$MCP_PROG" <<< "$REQUEST" 2>/dev/null || echo '{"error":"MCP call failed"}')

# Extract and parse the text response from MCP
# MCP returns {jsonrpc, result:{content:[{type,text}]}, id}
# The text field contains "exit_code: N" which we parse and return as JSON
if echo "$RESPONSE" | jq -e '.result.content[0].text' > /dev/null 2>&1; then
    TEXT=$(echo "$RESPONSE" | jq -r '.result.content[0].text')
    # Extract exit_code from text: "exit_code: 0" → 0
    EXIT_CODE=$(echo "$TEXT" | grep -o "exit_code: [0-9]*" | grep -o "[0-9]*" | head -1)
    # Return JSON with parsed exit_code (kernel code expects this format)
    printf '{"exit_code":%s}\n' "${EXIT_CODE:-1}"
else
    # Return error if MCP failed
    echo '{"error":"MCP call failed"}'
fi
