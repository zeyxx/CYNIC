#!/usr/bin/env bash
# CYNIC — Gemini CLI BeforeTool hook (coord claims)
# Auto-claims kernel source files before write/edit tools.
# Protocol: read JSON stdin → write JSON stdout.
#   deny:  {"hookSpecificOutput": {"decision": "deny", "reason": "..."}}
#   allow: {} or no output
set -euo pipefail

INPUT=$(cat)

source ~/.cynic-env 2>/dev/null || true
KERNEL_ADDR="${CYNIC_REST_ADDR:-}"
API_KEY="${CYNIC_API_KEY:-}"

if ! command -v jq &>/dev/null; then
    echo '{}' # allow — can't check without jq
    exit 0
fi

# Extract file path from tool input (Gemini uses toolInput.file_path or toolInput.path)
FILE_PATH=$(echo "$INPUT" | jq -r '
    .toolInput.file_path //
    .toolInput.path //
    .toolInput.filePath //
    .tool_input.file_path //
    empty' 2>/dev/null || true)

# No file path → not a file operation → allow
if [[ -z "$FILE_PATH" ]]; then
    echo '{}'
    exit 0
fi

# Only coord-claim kernel source files
case "$FILE_PATH" in
    *cynic-kernel/src/*) ;;
    *cynic-kernel/domains/*) ;;
    *cynic-kernel/tests/*) ;;
    *)
        echo '{}'
        exit 0
        ;;
esac

# No API key → degrade gracefully
if [[ -z "$KERNEL_ADDR" || -z "$API_KEY" ]]; then
    echo '{}' # allow — can't coord without auth
    exit 0
fi

# Derive agent ID from Gemini session (use PID as fallback)
AGENT_ID="gemini-${GEMINI_SESSION_ID:-$$}"
TARGET_FILE="${FILE_PATH#*cynic-kernel/}"

# Attempt claim
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
        echo '{}'
        exit 0
        ;;
    409)
        CONFLICT_MSG=$(jq -r '.error // "file claimed by another agent"' "$CLAIM_TMP" 2>/dev/null || echo "conflict")
        cat <<EOF
{"hookSpecificOutput": {"decision": "deny", "reason": "COORD CONFLICT: ${CONFLICT_MSG}. Use cynic_coord_who to see active claims."}}
EOF
        exit 0
        ;;
    000)
        # Kernel unreachable — degrade, allow
        echo '{}'
        exit 0
        ;;
    *)
        # Other errors — degrade, allow
        echo '{}'
        exit 0
        ;;
esac
