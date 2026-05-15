#!/usr/bin/env bash
# CYNIC — Dispatch session-init hook
# On session start, check if a dispatch exists for this scope/agent.
# If PROPOSED or COMPLETED, warn. If CLAIMED or WORKING, proceed.
set -euo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

# Extract scope and session_id
SCOPE=$(echo "$INPUT" | jq -r '.scope // empty' 2>/dev/null || true)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || true)

if [[ -z "$SCOPE" || -z "$SESSION_ID" ]]; then
    exit 0
fi

AGENT_ID="claude-${SESSION_ID:0:12}"
KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# Query dispatch for this scope
DISPATCH=$(curl -s --connect-timeout 2 --max-time 3 -X GET \
    "http://${KERNEL_ADDR}/agent-dispatch?scope=$(echo "$SCOPE" | jq -sRr @uri)" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    2>/dev/null || echo '{}')

STATUS=$(echo "$DISPATCH" | jq -r '.dispatch.status // empty' 2>/dev/null || true)

# Warn if dispatch exists and is in terminal state
if [[ "$STATUS" == "PROPOSED" ]]; then
    echo "⚠️  WARNING: Dispatch for scope '$SCOPE' is PROPOSED (PR open). Check if it's merged." >&2
elif [[ "$STATUS" == "COMPLETED" ]]; then
    echo "⚠️  WARNING: Dispatch for scope '$SCOPE' is COMPLETED. Consider closing PR if not merged." >&2
fi

exit 0
