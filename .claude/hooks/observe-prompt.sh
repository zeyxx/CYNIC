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

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

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

curl -s --max-time 2 -X POST "http://${KERNEL_ADDR}/observe" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    -d "$PAYLOAD" > /dev/null 2>&1 &

# ── L0→L1 Bridge: first prompt = human dispatch → mempool item ──
# K15 consumer: session-init.sh reads domain=mempool and injects into next session.
# Other cortex sessions see what's being worked on before picking tasks.
DISPATCH_MARKER="/tmp/cynic-sessions/${AGENT_ID}.dispatched"
if [[ ! -f "$DISPATCH_MARKER" ]]; then
    # First prompt of session — this IS the human dispatch (CLAUDE.md Rule 2)
    DISPATCH_PAYLOAD=$(jq -n \
        --arg tool "human_dispatch" \
        --arg target "$AGENT_ID" \
        --arg status "claimed" \
        --arg context "$PROMPT_TRUNC" \
        --arg domain "mempool" \
        --arg agent_id "$AGENT_ID" \
        --arg session_id "$SESSION_ID" \
        '{tool: $tool, target: $target, status: $status, context: $context,
          domain: $domain, agent_id: $agent_id, session_id: $session_id,
          tags: ["dispatch","l0-bridge"],
          consumer: "session-init", action: "inject active dispatches into next cortex session"}')

    curl -s --max-time 2 -X POST "http://${KERNEL_ADDR}/observe" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "$DISPATCH_PAYLOAD" > /dev/null 2>&1 &

    # Mark dispatched — subsequent prompts skip this block
    mkdir -p /tmp/cynic-sessions
    touch "$DISPATCH_MARKER"
fi

# Update coordination scope — preserves registered_at, fire-and-forget
if [[ -n "${CYNIC_REST_ADDR:-}" ]]; then
    SCOPE_PAYLOAD=$(jq -n \
        --arg agent_id "$AGENT_ID" \
        --arg scope "$PROMPT_TRUNC" \
        '{agent_id: $agent_id, scope: $scope}')
    curl -s --max-time 2 -X POST "http://${KERNEL_ADDR}/coord/scope" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "$SCOPE_PAYLOAD" > /dev/null 2>&1 &

    # Phase 1 measurement: detect scope overlap and observe to mempool
    WHO_JSON=$(curl -s -H "Authorization: Bearer ${API_KEY}" "http://${KERNEL_ADDR}/coord/who" 2>/dev/null || echo '{}')
    OVERLAP_COUNT=$(echo "$WHO_JSON" | jq '[.agents[]? | select(.active == true) | .scope // "" | select(length > 10)] | length' 2>/dev/null || echo 0)
    if [[ "$OVERLAP_COUNT" -ge 2 ]]; then
        OVERLAP_LIST=$(echo "$WHO_JSON" | jq -r '.agents[]? | select(.active == true and (.scope // "" | length > 10)) | .agent_id + ": " + (.scope | .[0:50]) + "..."' | paste -sd '|' - 2>/dev/null || echo "")
        MEAS_PAYLOAD=$(jq -n \
            --arg tool "phase1_overlap" \
            --arg target "coordination" \
            --arg status "detected" \
            --arg context "Active: $OVERLAP_COUNT agents | $OVERLAP_LIST" \
            --arg domain "coordination" \
            --arg agent_id "$AGENT_ID" \
            '{tool: $tool, target: $target, status: $status, context: $context, domain: $domain, agent_id: $agent_id, tags: ["phase1-measurement"]}')
        curl -s --max-time 2 -X POST "http://${KERNEL_ADDR}/observe" \
            -H "Content-Type: application/json" \
            ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
            -d "$MEAS_PAYLOAD" > /dev/null 2>&1 &
    fi
fi

exit 0
