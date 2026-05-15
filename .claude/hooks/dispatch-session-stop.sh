#!/usr/bin/env bash
# CYNIC — Dispatch session-stop hook
# On session end, mark active dispatches COMPLETED.
set -euo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || true)

if [[ -z "$SESSION_ID" ]]; then
    exit 0
fi

AGENT_ID="claude-${SESSION_ID:0:12}"
KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# Query active dispatches for this agent
DISPATCHES=$(curl -s --connect-timeout 2 --max-time 3 -X GET \
    "http://${KERNEL_ADDR}/agent-dispatch?claimed_by=$(echo "$AGENT_ID" | jq -sRr @uri)" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    2>/dev/null || echo '{}')

# For each WORKING dispatch, mark COMPLETED
echo "$DISPATCHES" | jq -r '.dispatches[]? | select(.status == "WORKING") | .id' 2>/dev/null | while read -r DISPATCH_ID; do
    if [[ -n "$DISPATCH_ID" ]]; then
        curl -s --connect-timeout 2 --max-time 3 -X PUT \
            "http://${KERNEL_ADDR}/agent-dispatch/${DISPATCH_ID}/status" \
            -H "Content-Type: application/json" \
            ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
            -d '{"status":"COMPLETED"}' \
            2>/dev/null || true
    fi
done

exit 0
