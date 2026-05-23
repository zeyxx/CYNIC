#!/usr/bin/env bash
# CYNIC — UserPromptSubmit observer (SIMPLIFIED)
# Only produces the mempool dispatch (first prompt = human scope declaration).
# K15 consumer: session-init.sh reads domain=mempool to inject active dispatches.
# All per-prompt observation removed — Claude Code datasets are the SSOT.
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

# ── L0→L1 Bridge: first prompt = human dispatch → mempool item ──
# K15 consumer: session-init.sh reads domain=mempool and injects into next session.
# Other cortex sessions see what's being worked on before picking tasks.
DISPATCH_MARKER="/tmp/cynic-sessions/${AGENT_ID}.dispatched"
if [[ ! -f "$DISPATCH_MARKER" ]]; then
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
fi

exit 0
