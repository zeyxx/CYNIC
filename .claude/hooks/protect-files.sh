#!/usr/bin/env bash
# CYNIC — PreToolUse hook
# 1. Blocks edits to sensitive files
# 2. Detects secret patterns in commands
# 3. Auto-claims kernel files on Edit/Write — blocks only on real CONFLICT
set -euo pipefail

INPUT=$(cat)

# Guard: jq is required for all checks
if ! command -v jq &>/dev/null; then
    exit 0
fi

TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null || true)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null || true)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null || true)

# ── Protect sensitive files from Edit/Write ──
if [[ "$TOOL_NAME" == "Edit" || "$TOOL_NAME" == "Write" ]]; then
    case "$FILE_PATH" in
        */.ssh/*)
            echo "BLOCKED: cannot edit SSH keys ($FILE_PATH)" >&2; exit 2 ;;
        */.config/cynic/env|*/.config/cynic/llama-api-key)
            echo "BLOCKED: cannot edit secret config ($FILE_PATH)" >&2; exit 2 ;;
        */.env|*/.env.*)
            echo "BLOCKED: cannot edit env files ($FILE_PATH)" >&2; exit 2 ;;
        # .git/hooks and settings.local.json: no longer blocked.
        # Git hooks need maintenance. settings.local.json = permissions only.
    esac
fi

# ── Protect sensitive files from Read ──
if [[ "$TOOL_NAME" == "Read" ]]; then
    case "$FILE_PATH" in
        */.cynic-env|*/.config/cynic/env|*/.config/cynic/llama-api-key)
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

# ── Auto-claim for kernel code edits ──
# On Edit/Write to cynic-kernel/src/*, auto-claim the file transparently.
# Only BLOCK on real CONFLICT (another agent holds the file).
# Kernel down → allow (graceful degradation).
if [[ ("$TOOL_NAME" == "Edit" || "$TOOL_NAME" == "Write") && "$FILE_PATH" == *cynic-kernel/src/* ]]; then
    source ~/.cynic-env 2>/dev/null || true
    KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
    API_KEY="${CYNIC_API_KEY:-}"

    # Derive agent_id from session_id
    SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
    if [[ -z "$SESSION_ID" ]]; then
        exit 0  # No session → allow (graceful degradation)
    fi
    AGENT_ID="claude-${SESSION_ID:0:12}"

    # Use path relative to cynic-kernel/src/ — not basename.
    # "api/rest/judge.rs" and "judge.rs" are different files, different claims.
    TARGET_FILE="${FILE_PATH#*cynic-kernel/src/}"

    # Step 1: PRE-CHECK — does another agent already hold this file?
    # Short 1s timeout — fail-open if coord unreachable or slow.
    WHO_TMP=$(mktemp /tmp/cynic-who-XXXXXX)
    WHO_CODE=$(curl -s -o "$WHO_TMP" -w '%{http_code}' \
        --connect-timeout 1 --max-time 1 \
        "http://${KERNEL_ADDR}/agents" \
        ${API_KEY:+-H "Authorization: Bearer $API_KEY"} \
        2>/dev/null || echo "000")

    if [[ "$WHO_CODE" == "200" ]]; then
        HOLDER=$(jq -r --arg target "$TARGET_FILE" --arg self "$AGENT_ID" \
            '.claims[]? | select(.target == $target and .active == true and .agent_id != $self) | .agent_id' \
            "$WHO_TMP" 2>/dev/null | head -1)
        if [[ -n "$HOLDER" ]]; then
            rm -f "$WHO_TMP"
            echo "BLOCKED: CONFLICT: '${TARGET_FILE}' already claimed by ${HOLDER}" >&2
            exit 2
        fi
    fi
    rm -f "$WHO_TMP"

    # Step 2: CLAIM — POST /coord/claim (authoritative gate)
    # Returns 200 {"status":"claimed"} or 409 {"error":"CONFLICT: ..."}
    CLAIM_TMP=$(mktemp /tmp/cynic-claim-XXXXXX)
    trap "rm -f '$CLAIM_TMP'" EXIT
    HTTP_CODE=$(curl -s -o "$CLAIM_TMP" -w '%{http_code}' \
        --connect-timeout 2 --max-time 3 \
        -X POST "http://${KERNEL_ADDR}/coord/claim" \
        -H "Content-Type: application/json" \
        ${API_KEY:+-H "Authorization: Bearer $API_KEY"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"target\":\"${TARGET_FILE}\",\"claim_type\":\"file\"}" \
        2>/dev/null || echo "000")

    if [[ "$HTTP_CODE" == "000" ]]; then
        exit 0  # Kernel unreachable → allow (graceful degradation)
    fi

    if [[ "$HTTP_CODE" == "409" ]]; then
        CONFLICT_MSG=$(jq -r '.error // "conflict"' "$CLAIM_TMP" 2>/dev/null || echo "conflict")
        echo "BLOCKED: ${CONFLICT_MSG}" >&2
        exit 2
    fi

    # 200 (claimed), 401 (auth issue), 500 (DB down) — all allow through
    # Only a real CONFLICT blocks. Everything else degrades gracefully.
fi

# All clear
exit 0
