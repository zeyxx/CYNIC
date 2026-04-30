#!/usr/bin/env bash
# CYNIC — UserPromptSubmit observer
# Captures the human's questions/instructions → POST /observe → SurrealDB → CCM
# K15 consumer: CCM crystallizes reasoning patterns from prompt→action sequences.
# Fire-and-forget: async, never blocks the session.
set -uo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

PROMPT=$(echo "$INPUT" | jq -r '.prompt // empty')
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
AGENT_ID="claude-${SESSION_ID:0:12}"
[[ "$SESSION_ID" == "" ]] && AGENT_ID="unknown"

# Skip empty prompts and very short ones (slash commands, "y", "go")
[[ ${#PROMPT} -lt 5 ]] && exit 0

# Truncate prompt to budget (400 chars — enough for intent, not full text)
PROMPT_TRUNC="${PROMPT:0:400}"

PAYLOAD=$(jq -n \
    --arg tool "user_prompt" \
    --arg target "session" \
    --arg status "ok" \
    --arg context "$PROMPT_TRUNC" \
    --arg domain "session" \
    --arg agent_id "$AGENT_ID" \
    --arg session_id "$SESSION_ID" \
    '{tool: $tool, target: $target, status: $status, context: $context,
      domain: $domain, agent_id: $agent_id, session_id: $session_id,
      tags: ["user-prompt","reasoning-trail"]}')

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

curl -s --max-time 2 -X POST "http://${KERNEL_ADDR}/observe" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    -d "$PAYLOAD" > /dev/null 2>&1 &

exit 0
