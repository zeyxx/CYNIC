#!/usr/bin/env bash
# CYNIC — PreToolUse hook
# 1. Blocks edits to sensitive files
# 2. Detects secret patterns in commands
# 3. Verifies coord claims before Edit/Write on kernel code
set -euo pipefail

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# ── Protect sensitive files from Edit/Write ──
if [[ "$TOOL_NAME" == "Edit" || "$TOOL_NAME" == "Write" || "$TOOL_NAME" == "MultiEdit" ]]; then
    case "$FILE_PATH" in
        */.ssh/*)
            echo "BLOCKED: cannot edit SSH keys ($FILE_PATH)" >&2; exit 2 ;;
        */.config/cynic/env|*/.config/cynic/llama-api-key)
            echo "BLOCKED: cannot edit secret config ($FILE_PATH)" >&2; exit 2 ;;
        */.env|*/.env.*)
            echo "BLOCKED: cannot edit env files ($FILE_PATH)" >&2; exit 2 ;;
        */.git/hooks/*)
            echo "BLOCKED: cannot modify git hooks — edit manually ($FILE_PATH)" >&2; exit 2 ;;
        */.claude/settings.local.json)
            echo "BLOCKED: cannot self-modify settings — edit manually ($FILE_PATH)" >&2; exit 2 ;;
    esac
fi

# ── Protect sensitive files from Read ──
if [[ "$TOOL_NAME" == "Read" ]]; then
    case "$FILE_PATH" in
        */.config/cynic/env|*/.config/cynic/llama-api-key)
            echo "BLOCKED: cannot read secret config ($FILE_PATH)" >&2; exit 2 ;;
        */.ssh/id_*|*/.ssh/known_hosts)
            echo "BLOCKED: cannot read SSH keys ($FILE_PATH)" >&2; exit 2 ;;
    esac
fi

# ── Detect secrets in Bash commands ──
if [[ "$TOOL_NAME" == "Bash" && -n "$COMMAND" ]]; then
    if echo "$COMMAND" | grep -qiE '(AIzaSy[A-Za-z0-9_-]{30}|hf_[A-Za-z0-9]{30}|sk-[A-Za-z0-9]{40})'; then
        echo "BLOCKED: command contains what looks like a real API key" >&2; exit 2
    fi
fi

# ── Coord claim verification for kernel code edits ──
# Only check for Edit/Write on cynic-kernel/ source files
if [[ ("$TOOL_NAME" == "Edit" || "$TOOL_NAME" == "Write") && "$FILE_PATH" == *cynic-kernel/src/* ]]; then
    source ~/.cynic-env 2>/dev/null || true
    KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
    API_KEY="${CYNIC_API_KEY:-}"

    # Derive agent_id from session_id (same as session-init.sh)
    SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
    if [[ -n "$SESSION_ID" ]]; then
        AGENT_ID="claude-${SESSION_ID:0:12}"
    else
        # Can't verify without agent_id — allow (graceful degradation)
        exit 0
    fi

    # Check current claims via GET /agents
    AGENTS_JSON=$(curl -s --max-time 2 "http://${KERNEL_ADDR}/agents" \
        ${API_KEY:+-H "Authorization: Bearer $API_KEY"} 2>/dev/null || echo '{}')

    # Extract the filename from the path (last component)
    TARGET_FILE=$(basename "$FILE_PATH")

    # Check if this agent has ANY active claim (lightweight check)
    HAS_CLAIM=$(echo "$AGENTS_JSON" | jq -r \
        --arg agent "$AGENT_ID" \
        '.claims // [] | map(select(.agent_id == $agent and .active == true)) | length' \
        2>/dev/null || echo "0")

    if [[ "$HAS_CLAIM" == "0" ]]; then
        # Output JSON to deny with reason — Claude sees this as feedback
        cat <<DENY_JSON
{
    "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "allow",
        "additionalContext": "WARNING: No coord claim found for agent ${AGENT_ID}. You should call cynic_coord_claim(agent_id=\"${AGENT_ID}\", target=\"${TARGET_FILE}\") before editing kernel code. Coordination prevents multi-agent conflicts."
    }
}
DENY_JSON
        exit 0
    fi
fi

# All clear
exit 0
