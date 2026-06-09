#!/usr/bin/env bash
# CYNIC — Gemini CLI AfterTool hook
# Mirrors Claude's observe-tool.sh: POST tool usage to /observe.
# Fire-and-forget, async. stdout MUST be pure JSON.
set -uo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null || true)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || true)
AGENT_ID="gemini-${SESSION_ID:0:12}"
[[ -z "$SESSION_ID" ]] && AGENT_ID="gemini-unknown"

# Skip noisy tools
case "$TOOL_NAME" in
    write_file|edit_file|replace_in_file|apply_diff|read_file|shell|grep|glob|find_files) ;;
    *) echo '{}'; exit 0 ;;
esac

# Extract target
TARGET=""
case "$TOOL_NAME" in
    write_file|edit_file|replace_in_file|apply_diff|read_file)
        TARGET=$(echo "$INPUT" | jq -r '.tool_input.file_path // .tool_input.path // empty' 2>/dev/null | head -c 120)
        ;;
    shell)
        TARGET=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null | head -c 80)
        ;;
    grep|find_files|glob)
        TARGET=$(echo "$INPUT" | jq -r '.tool_input.pattern // .tool_input.query // empty' 2>/dev/null | head -c 80)
        ;;
esac

# Map Gemini tool names to canonical names for domain derivation
CANONICAL_TOOL="$TOOL_NAME"
case "$TOOL_NAME" in
    write_file) CANONICAL_TOOL="Write" ;;
    edit_file|replace_in_file|apply_diff) CANONICAL_TOOL="Edit" ;;
    read_file) CANONICAL_TOOL="Read" ;;
    shell) CANONICAL_TOOL="Bash" ;;
    grep) CANONICAL_TOOL="Grep" ;;
    glob|find_files) CANONICAL_TOOL="Glob" ;;
esac

# Derive domain
DOMAIN="general"
case "$TARGET" in
    */crates/cynic-kernel/*) DOMAIN="rust" ;;
    */services/cynic-python/*) DOMAIN="python" ;;
    */.gemini/*|*/.claude/*) DOMAIN="harness" ;;
    */docs/*) DOMAIN="docs" ;;
    */scripts/*) DOMAIN="ops" ;;
esac
if [[ "$TOOL_NAME" == "shell" ]]; then
    case "$TARGET" in
        cargo*|make*) DOMAIN="rust" ;;
        python3*|pip*) DOMAIN="python" ;;
        git*|gh*) DOMAIN="git" ;;
        curl*|systemctl*) DOMAIN="ops" ;;
    esac
fi

# Derive tags
TAGS='[]'
case "$CANONICAL_TOOL" in
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

# Status from tool response
STATUS="success"
TOOL_OUTPUT=$(echo "$INPUT" | jq -r '.tool_response.llmContent // .tool_response.error // empty' 2>/dev/null | head -c 200 || true)
if echo "$TOOL_OUTPUT" | grep -qiE '(error|BLOCKED|failed|SIGSEGV|panic)' 2>/dev/null; then
    STATUS="error"
fi

# Build and POST (fire-and-forget in background)
AUTH_HDR=""
[ -n "$API_KEY" ] && AUTH_HDR="Authorization: Bearer $API_KEY"

PAYLOAD=$(jq -n \
    --arg tool "$CANONICAL_TOOL" \
    --arg target "$TARGET" \
    --arg status "$STATUS" \
    --arg context "${TOOL_OUTPUT:0:200}" \
    --arg domain "$DOMAIN" \
    --arg agent_id "$AGENT_ID" \
    --arg session_id "$SESSION_ID" \
    --argjson tags "$TAGS" \
    '{tool: $tool, target: $target, status: $status, context: $context,
      domain: $domain, agent_id: $agent_id, session_id: $session_id, tags: $tags}')

curl -s --max-time 2 -X POST "http://${KERNEL_ADDR}/observe" \
    -H "Content-Type: application/json" \
    ${AUTH_HDR:+-H "$AUTH_HDR"} \
    -d "$PAYLOAD" > /dev/null 2>&1 &

# Heartbeat
curl -s --max-time 1 -X POST "http://${KERNEL_ADDR}/coord/heartbeat" \
    -H "Content-Type: application/json" \
    ${AUTH_HDR:+-H "$AUTH_HDR"} \
    -d "{\"agent_id\":\"${AGENT_ID}\"}" \
    > /dev/null 2>&1 &

# Gemini requires JSON stdout (empty object = no action)
echo '{}'
