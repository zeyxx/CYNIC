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
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
TOOL_USE_ID=$(echo "$INPUT" | jq -r '.tool_use_id // empty')
AGENT_ID="claude-${SESSION_ID:0:12}"
[[ "$SESSION_ID" == "" ]] && AGENT_ID="unknown"

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

# ── Derive domain from file path or command ──
DOMAIN="general"
case "$TARGET" in
    */cynic-kernel/*) DOMAIN="rust" ;;
    */cynic-python/*) DOMAIN="python" ;;
    */.claude/*) DOMAIN="harness" ;;
    */docs/*) DOMAIN="docs" ;;
    */scripts/*) DOMAIN="ops" ;;
esac
if [[ "$TOOL_NAME" == "Bash" ]]; then
    case "$TARGET" in
        cargo*|make*) DOMAIN="rust" ;;
        python3*|pip*) DOMAIN="python" ;;
        git*|gh*) DOMAIN="git" ;;
        curl*|systemctl*) DOMAIN="ops" ;;
    esac
fi

# ── Derive action tags ──
TAGS='[]'
case "$TOOL_NAME" in
    Edit) TAGS='["edit"]' ;;
    Write) TAGS='["write"]' ;;
    Read) TAGS='["read"]' ;;
    Bash)
        case "$TARGET" in
            cargo\ build*|cargo\ check*|make*) TAGS='["build"]' ;;
            cargo\ test*|pytest*) TAGS='["test"]' ;;
            cargo\ clippy*) TAGS='["lint"]' ;;
            git\ commit*|git\ push*|gh\ pr*) TAGS='["ship"]' ;;
            curl*) TAGS='["probe"]' ;;
            *) TAGS='["bash"]' ;;
        esac ;;
    Grep|Glob) TAGS='["search"]' ;;
esac

# Determine status from tool output (heuristic: check if error/blocked)
STATUS="success"
TOOL_OUTPUT=$(echo "$INPUT" | jq -r '.tool_output // empty' 2>/dev/null | head -c 200 || true)
if echo "$TOOL_OUTPUT" | grep -qiE '(error|BLOCKED|failed|SIGSEGV|panic)'; then
    STATUS="error"
fi

# Build JSON payload
PAYLOAD=$(jq -n \
    --arg tool "$TOOL_NAME" \
    --arg target "$TARGET" \
    --arg status "$STATUS" \
    --arg context "${TOOL_OUTPUT:0:200}" \
    --arg domain "$DOMAIN" \
    --arg agent_id "$AGENT_ID" \
    --arg session_id "$SESSION_ID" \
    --arg tool_use_id "$TOOL_USE_ID" \
    --argjson tags "$TAGS" \
    '{tool: $tool, target: $target, status: $status, context: $context,
      domain: $domain, agent_id: $agent_id, session_id: $session_id,
      tool_use_id: $tool_use_id, tags: $tags}')

# Fire-and-forget — POST in background, ignore result
AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

curl -s --max-time 2 -X POST "http://${KERNEL_ADDR}/observe" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    -d "$PAYLOAD" > /dev/null 2>&1 &

# ── Coord heartbeat (keep agent alive, prevents 5-min TTL expiry) ──
curl -s --max-time 1 -X POST "http://${KERNEL_ADDR}/coord/heartbeat" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    -d "{\"agent_id\":\"${AGENT_ID}\"}" \
    > /dev/null 2>&1 &

exit 0
