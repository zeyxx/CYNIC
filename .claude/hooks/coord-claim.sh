#!/usr/bin/env bash
# CYNIC — PreToolUse hook: data-centric zone activity check.
# Queries kernel for recent zone activity (from observation stream).
# Advisory only: warns on overlap, never blocks. Agent decides.
# No local state. No /tmp/ files. No TTL. Pure read.
set -euo pipefail

INPUT=$(cat)

FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null || true)
[[ -z "$FILE_PATH" ]] && exit 0

SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
source ~/.cynic-env 2>/dev/null || true
KERNEL_ADDR="${CYNIC_REST_ADDR:-127.0.0.1:3030}"
API_KEY="${CYNIC_API_KEY:-}"
[[ -z "$API_KEY" ]] && exit 0

# Derive agent_id from session
AGENT_ID=""
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    SESSION_STATE_DIR="/tmp/cynic-sessions"
    if [[ -d "$SESSION_STATE_DIR" ]]; then
        RECENT_STATE=$(ls -t "$SESSION_STATE_DIR"/*.state 2>/dev/null | head -1)
        if [[ -n "${RECENT_STATE:-}" ]]; then
            AGENT_ID=$(grep -oP 'agent_id=\K[^ ]+' "$RECENT_STATE" 2>/dev/null || true)
        fi
    fi
fi
[[ -z "$AGENT_ID" ]] && exit 0

# Strip project root for relative path
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
REL_PATH="${FILE_PATH#$PROJECT_DIR/}"

# Query kernel: who else is active in this zone? (single GET, no state mutation)
RESP=$(curl -s --connect-timeout 2 --max-time 3 \
    "http://${KERNEL_ADDR}/dispatch/zone-activity?file_path=$(printf '%s' "$REL_PATH" | jq -sRr @uri)&agent_id=${AGENT_ID}" \
    -H "Authorization: Bearer $API_KEY" \
    2>/dev/null || echo "")

# Interpret response — advisory only, never block
if [[ -n "$RESP" ]]; then
    STATUS=$(echo "$RESP" | jq -r '.zone // empty' 2>/dev/null)
    if [[ -n "$STATUS" ]]; then
        AGENTS=$(echo "$RESP" | jq -r '.active_agents[].agent_id' 2>/dev/null | head -3)
        if [[ -n "$AGENTS" ]]; then
            LAST=$(echo "$RESP" | jq -r '.active_agents[0].last_active // ""' 2>/dev/null)
            echo "⚠ ZONE '${STATUS}': also active — ${AGENTS//$'\n'/, } (last: ${LAST})" >&2
        fi
    fi
fi

exit 0
