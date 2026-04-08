#!/usr/bin/env bash
# CYNIC — PreToolUse hook
# 1. Blocks edits to sensitive files
# 2. Detects secret patterns in commands
# 3. Auto-claims kernel files on Edit/Write — fail-closed by default
set -euo pipefail

INPUT=$(cat)

block() {
    echo "BLOCKED: $*" >&2
    exit 2
}

is_truthy() {
    case "${1:-}" in
        1|true|TRUE|yes|YES) return 0 ;;
        *) return 1 ;;
    esac
}

coord_unavailable() {
    local reason="$1"
    if is_truthy "${CYNIC_COORD_ALLOW_DEGRADED:-}"; then
        echo "WARN: degraded coordination override — $reason" >&2
        exit 0
    fi
    block "$reason"
}

# Guard: jq is required to parse hook payloads safely. Missing jq = no enforceable policy.
if ! command -v jq &>/dev/null; then
    block "jq is required for protect-files.sh"
fi

TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null || true)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null || true)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null || true)

# ── Protect sensitive files from Edit/Write ──
if [[ "$TOOL_NAME" == "Edit" || "$TOOL_NAME" == "Write" ]]; then
    case "$FILE_PATH" in
        */.ssh/*)
            block "cannot edit SSH keys ($FILE_PATH)" ;;
        */.config/cynic/env|*/.config/cynic/llama-api-key)
            block "cannot edit secret config ($FILE_PATH)" ;;
        */.env|*/.env.*)
            block "cannot edit env files ($FILE_PATH)" ;;
        # .git/hooks and settings.local.json: no longer blocked.
        # Git hooks need maintenance. settings.local.json = permissions only.
    esac
fi

# ── Protect sensitive files from Read ──
if [[ "$TOOL_NAME" == "Read" ]]; then
    case "$FILE_PATH" in
        */.cynic-env|*/.config/cynic/env|*/.config/cynic/llama-api-key)
            block "cannot read secret config ($FILE_PATH)" ;;
        */.ssh/id_*|*/.ssh/known_hosts)
            block "cannot read SSH keys ($FILE_PATH)" ;;
    esac
fi

# ── Detect secrets in Bash commands ──
if [[ "$TOOL_NAME" == "Bash" && -n "$COMMAND" ]]; then
    if echo "$COMMAND" | grep -qiE '(AIzaSy[A-Za-z0-9_-]{30}|hf_[A-Za-z0-9]{30}|sk-[A-Za-z0-9]{40})'; then
        block "command contains what looks like a real API key"
    fi
fi

# ── Auto-claim for kernel code edits ──
# Policy:
# - Sensitive files/secrets are always fail-closed.
# - Writes to cynic-kernel/src/* must reach the authoritative /coord/claim gate.
# - Kernel/auth outages stay blocked unless an operator explicitly sets
#   CYNIC_COORD_ALLOW_DEGRADED=1 as a breakglass override.
if [[ ("$TOOL_NAME" == "Edit" || "$TOOL_NAME" == "Write") && "$FILE_PATH" == *cynic-kernel/src/* ]]; then
    source ~/.cynic-env 2>/dev/null || true
    KERNEL_ADDR="${CYNIC_REST_ADDR:-127.0.0.1:3030}"
    API_KEY="${CYNIC_API_KEY:-}"

    # Derive agent_id from session_id
    SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
    if [[ -z "$SESSION_ID" ]]; then
        block "coordination requires session_id for kernel edits"
    fi
    if [[ -z "$API_KEY" ]]; then
        block "coordination requires CYNIC_API_KEY for kernel edits"
    fi
    AGENT_ID="claude-${SESSION_ID:0:12}"

    # Use path relative to cynic-kernel/src/ — not basename.
    # "api/rest/judge.rs" and "judge.rs" are different files, different claims.
    TARGET_FILE="${FILE_PATH#*cynic-kernel/src/}"

    # Authoritative claim gate — no fail-open pre-check.
    # Returns 200 {"status":"claimed"} or 409 {"error":"CONFLICT: ..."}
    CLAIM_TMP=$(mktemp /tmp/cynic-claim-XXXXXX)
    trap "rm -f '$CLAIM_TMP'" EXIT
    HTTP_CODE=$(curl -s -o "$CLAIM_TMP" -w '%{http_code}' \
        --connect-timeout 2 --max-time 3 \
        -X POST "http://${KERNEL_ADDR}/coord/claim" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"target\":\"${TARGET_FILE}\",\"claim_type\":\"file\"}" \
        2>/dev/null || echo "000")

    case "$HTTP_CODE" in
        200)
            exit 0
            ;;
        409)
            CONFLICT_MSG=$(jq -r '.error // "conflict"' "$CLAIM_TMP" 2>/dev/null || echo "conflict")
            block "$CONFLICT_MSG"
            ;;
        000)
            coord_unavailable "coordination unavailable while claiming '$TARGET_FILE'"
            ;;
        401|403)
            block "coordination auth failed for '$TARGET_FILE' (HTTP $HTTP_CODE)"
            ;;
        *)
            coord_unavailable "coordination claim failed for '$TARGET_FILE' (HTTP $HTTP_CODE)"
            ;;
    esac
fi

# All clear
exit 0
