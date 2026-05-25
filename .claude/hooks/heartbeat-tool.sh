#!/usr/bin/env bash
# CYNIC — PostToolUse: fire-and-forget heartbeat to keep session alive.
# Prevents 5-min TTL expiry during long tool sequences.
# No observation POST — K15: no consumer for tool observations.
set -uo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
AGENT_ID="claude-${SESSION_ID:0:12}"
[[ "$SESSION_ID" == "" ]] && exit 0

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# Fire-and-forget heartbeat — async, 1s timeout, ignore result
curl -s --max-time 1 -X POST "http://${KERNEL_ADDR}/coord/heartbeat" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    -d "{\"agent_id\":\"${AGENT_ID}\"}" \
    > /dev/null 2>&1 &

exit 0
