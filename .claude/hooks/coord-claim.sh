#!/usr/bin/env bash
# CYNIC — PreToolUse hook (coord claims only)
# Auto-claims kernel source files on Edit/Write.
# Scoped via settings.json `if` field to cynic-kernel/src/* — only spawns for kernel edits.
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

if ! command -v jq &>/dev/null; then
    block "jq is required for coord-claim.sh"
fi

FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null || true)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')

source ~/.cynic-env 2>/dev/null || true
KERNEL_ADDR="${CYNIC_REST_ADDR:-127.0.0.1:3030}"
API_KEY="${CYNIC_API_KEY:-}"

if [[ -z "$API_KEY" ]]; then
    block "coordination requires CYNIC_API_KEY for kernel edits"
fi

# Derive AGENT_ID from SESSION_ID (if available from Claude context)
# Fallback: read from most recent session state file (set by session-init.sh)
AGENT_ID=""
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    # Fallback: find most recent session state file
    SESSION_STATE_DIR="/tmp/cynic-sessions"
    if [[ -d "$SESSION_STATE_DIR" ]]; then
        RECENT_STATE=$(ls -t "$SESSION_STATE_DIR"/*.state 2>/dev/null | head -1)
        if [[ -n "$RECENT_STATE" ]]; then
            AGENT_ID=$(grep -oP 'agent_id=\K[^ ]+' "$RECENT_STATE" 2>/dev/null || true)
        fi
    fi
fi

if [[ -z "$AGENT_ID" ]]; then
    block "coordination requires valid AGENT_ID (SESSION_ID missing and no session state file found)"
fi
TARGET_FILE="${FILE_PATH#*cynic-kernel/src/}"

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
        # Agent Protocol v1: check for conflicts in response (signal, not lock)
        CONFLICTS=$(jq -r '.conflicts // empty' "$CLAIM_TMP" 2>/dev/null)
        if [ -n "$CONFLICTS" ] && [ "$CONFLICTS" != "null" ] && [ "$CONFLICTS" != "[]" ]; then
            CONFLICT_AGENTS=$(jq -r '.conflicts[].agent_id' "$CLAIM_TMP" 2>/dev/null | tr '\n' ', ')
            echo "⚠ SCOPE OVERLAP: '$TARGET_FILE' also claimed by: ${CONFLICT_AGENTS%,}" >&2
            echo "  Your work may conflict. Check /coord/who for details." >&2
        fi
        exit 0
        ;;
    409)
        # Legacy path — should not happen after REST change, warn instead of block
        CONFLICT_MSG=$(jq -r '.error // "conflict"' "$CLAIM_TMP" 2>/dev/null || echo "conflict")
        echo "⚠ CLAIM CONFLICT (legacy 409): $CONFLICT_MSG" >&2
        exit 0
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
