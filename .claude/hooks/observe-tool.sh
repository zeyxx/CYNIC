#!/usr/bin/env bash
# CYNIC — PostToolUse hook
# Captures tool usage as observations → POST /observe → SurrealDB → CCM
# Fire-and-forget: runs curl in background, never blocks the session.
set -uo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

# Skip noisy/meta tools — only observe substantive actions
case "$TOOL_NAME" in
    Edit|Write|Bash|Read|Grep|Glob|NotebookEdit) ;;
    *) exit 0 ;;
esac

# Extract target (file_path for Edit/Write/Read, command for Bash, pattern for Grep/Glob)
TARGET=""
case "$TOOL_NAME" in
    Edit|Write|Read)
        TARGET=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
        ;;
    Bash)
        # First 80 chars of command — enough to identify the action
        TARGET=$(echo "$INPUT" | jq -r '.tool_input.command // empty' | head -c 80)
        ;;
    Grep)
        TARGET=$(echo "$INPUT" | jq -r '.tool_input.pattern // empty')
        ;;
    Glob)
        TARGET=$(echo "$INPUT" | jq -r '.tool_input.pattern // empty')
        ;;
esac

# Determine status from tool output (heuristic: check if error/blocked)
STATUS="success"
TOOL_OUTPUT=$(echo "$INPUT" | jq -r '.tool_output // empty' | head -c 200)
if echo "$TOOL_OUTPUT" | grep -qiE '(error|BLOCKED|failed|SIGSEGV|panic)'; then
    STATUS="error"
fi

# Build JSON payload
PAYLOAD=$(jq -n \
    --arg tool "$TOOL_NAME" \
    --arg target "$TARGET" \
    --arg status "$STATUS" \
    --arg context "${TOOL_OUTPUT:0:200}" \
    --arg agent_id "$(cat /tmp/cynic-agent-id 2>/dev/null || echo unknown)" \
    --arg session_id "${CYNIC_SESSION_ID:-}" \
    '{tool: $tool, target: $target, status: $status, context: $context, agent_id: $agent_id, session_id: $session_id}')

# Fire-and-forget — POST in background, ignore result
AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

curl -s --max-time 2 -X POST "http://${KERNEL_ADDR}/observe" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    -d "$PAYLOAD" > /dev/null 2>&1 &

exit 0
