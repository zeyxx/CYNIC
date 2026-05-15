#!/usr/bin/env bash
# CYNIC — Dispatch coord-claim validation
# Before claiming a file, check if dispatch exists and is WORKING or CLAIMED.
# Block edits if PROPOSED/COMPLETED (post-merge gatefold).
set -euo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

FILE_PATH=$(echo "$INPUT" | jq -r '.file_path // .path // empty' 2>/dev/null || true)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || true)

if [[ -z "$FILE_PATH" || -z "$SESSION_ID" ]]; then
    exit 0
fi

AGENT_ID="claude-${SESSION_ID:0:12}"
KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# Query dispatch for this agent
DISPATCHES=$(curl -s --connect-timeout 2 --max-time 3 -X GET \
    "http://${KERNEL_ADDR}/agent-dispatch?claimed_by=$(echo "$AGENT_ID" | jq -sRr @uri)" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    2>/dev/null || echo '{}')

# Check if any active dispatch is in terminal state
TERMINAL_COUNT=$(echo "$DISPATCHES" | jq '[.dispatches[]? | select(.status == "PROPOSED" or .status == "COMPLETED")] | length' 2>/dev/null || echo 0)

if [[ "$TERMINAL_COUNT" -gt 0 ]]; then
    echo "ERROR: Cannot edit while dispatch is in PROPOSED or COMPLETED state (PR pending/merged)." >&2
    exit 425  # Too Early
fi

exit 0
