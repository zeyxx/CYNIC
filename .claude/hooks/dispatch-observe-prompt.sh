#!/usr/bin/env bash
# CYNIC — Dispatch observe-prompt hook
# On first user message, create a dispatch record if none exists.
# Extracts scope from message content.
set -euo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

MESSAGE=$(echo "$INPUT" | jq -r '.content // .text // empty' 2>/dev/null || true)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || true)

if [[ -z "$MESSAGE" || -z "$SESSION_ID" ]]; then
    exit 0
fi

AGENT_ID="claude-${SESSION_ID:0:12}"
KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# Extract scope from first line of message (heuristic: word before first colon or dash)
SCOPE=$(echo "$MESSAGE" | head -1 | grep -oE '^[a-zA-Z0-9_-]+' | head -1 || true)

if [[ -z "$SCOPE" ]]; then
    exit 0
fi

# Check if dispatch already exists for this scope
EXISTING=$(curl -s --connect-timeout 2 --max-time 3 -X GET \
    "http://${KERNEL_ADDR}/agent-dispatch?scope=$(echo "$SCOPE" | jq -sRr @uri)" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    2>/dev/null || echo '{}')

EXISTING_STATUS=$(echo "$EXISTING" | jq -r '.dispatch.status // empty' 2>/dev/null || true)

if [[ -n "$EXISTING_STATUS" ]]; then
    # Dispatch exists, skip creation
    exit 0
fi

# Get current branch
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "main")

# Get primary zone from first file in scope
ZONE=$(echo "$MESSAGE" | grep -oE 'cynic-kernel/src/[a-z_/]+' | head -1 | sed 's|cynic-kernel/src/||' | cut -d'/' -f1 || echo "api")

# Create new dispatch
curl -s --connect-timeout 2 --max-time 3 -X POST \
    "http://${KERNEL_ADDR}/agent-dispatch" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    -d "{
        \"scope\":\"${SCOPE}\",
        \"zone\":\"${ZONE}\",
        \"claimed_by\":\"${AGENT_ID}\",
        \"branch\":\"${BRANCH}\"
    }" \
    2>/dev/null || true

exit 0
