#!/usr/bin/env bash
# CYNIC — Dispatch branch-guard
# Block edits on branches where dispatch is PROPOSED or COMPLETED.
# K15 consumer: prevents editing merged branches or open PRs.
set -euo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

FILE_PATH=$(echo "$INPUT" | jq -r '.file_path // .path // empty' 2>/dev/null || true)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || true)

if [[ -z "$FILE_PATH" || -z "$SESSION_ID" ]]; then
    exit 0
fi

AGENT_ID="claude-${SESSION_ID:0:12}"
KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# Get current branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

if [[ -z "$CURRENT_BRANCH" ]]; then
    exit 0
fi

# Query dispatch by branch name
DISPATCH=$(curl -s --connect-timeout 2 --max-time 3 -X GET \
    "http://${KERNEL_ADDR}/agent-dispatch?scope=$(echo "$CURRENT_BRANCH" | jq -sRr @uri)" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    2>/dev/null || echo '{}')

STATUS=$(echo "$DISPATCH" | jq -r '.dispatch.status // empty' 2>/dev/null || true)

# Block if dispatch exists and is terminal (PR open or merged)
if [[ "$STATUS" == "PROPOSED" ]]; then
    echo "ERROR: Cannot edit branch with PROPOSED dispatch (PR open). Wait for merge or close PR." >&2
    exit 423  # Locked
elif [[ "$STATUS" == "COMPLETED" ]]; then
    echo "ERROR: Cannot edit branch with COMPLETED dispatch (PR merged). Use main or new branch." >&2
    exit 423  # Locked
fi

exit 0
