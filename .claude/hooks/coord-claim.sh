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

if [[ -z "$SESSION_ID" ]]; then
    block "coordination requires session_id for kernel edits"
fi
if [[ -z "$API_KEY" ]]; then
    block "coordination requires CYNIC_API_KEY for kernel edits"
fi

AGENT_ID="claude-${SESSION_ID:0:12}"
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
