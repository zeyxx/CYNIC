#!/usr/bin/env bash
# CYNIC — SubagentStart observer
# Captures agent dispatches → POST /observe → SurrealDB → CCM
# K15: organism sees what reasoning agents were spawned and why.
# Fire-and-forget: async, never blocks.
set -uo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

AGENT_TYPE=$(echo "$INPUT" | jq -r '.agent_type // "unknown"')
PROMPT_SUMMARY=$(echo "$INPUT" | jq -r '.prompt // empty' | head -c 300)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
AGENT_ID="claude-${SESSION_ID:0:12}"
[[ "$SESSION_ID" == "" ]] && AGENT_ID="unknown"

PAYLOAD=$(jq -n \
    --arg tool "agent_dispatch" \
    --arg target "$AGENT_TYPE" \
    --arg status "ok" \
    --arg context "$PROMPT_SUMMARY" \
    --arg domain "session" \
    --arg agent_id "$AGENT_ID" \
    --arg session_id "$SESSION_ID" \
    '{tool: $tool, target: $target, status: $status, context: $context,
      domain: $domain, agent_id: $agent_id, session_id: $session_id,
      tags: ["agent-dispatch","reasoning-trail"]}')

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

curl -s --max-time 2 -X POST "http://${KERNEL_ADDR}/observe" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    -d "$PAYLOAD" > /dev/null 2>&1 &

exit 0
