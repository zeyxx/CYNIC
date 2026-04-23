#!/usr/bin/env bash
# CYNIC — SessionEnd hook (dream trigger)
# Records session end, increments counter.
# SessionStart (session-init.sh) checks conditions and suggests /dream.
# Mechanical — no LLM involved.
#
# Debounce: subagents also trigger Stop hooks, inflating the counter.
# Only increment if last increment was >60s ago (one real session = one count).
set -euo pipefail

MEMORY_DIR="${HOME}/.claude/projects/-home-user-Bureau-CYNIC/memory"
STATE_FILE="${MEMORY_DIR}/.dream-state"
LOCK_FILE="${MEMORY_DIR}/.dream-debounce"

# Initialize state file if missing
if [[ ! -f "$STATE_FILE" ]]; then
    cat > "$STATE_FILE" <<EOF
last_dream=$(date -Iseconds)
sessions_since=0
EOF
fi

# Debounce: skip if last increment was <60s ago
if [[ -f "$LOCK_FILE" ]]; then
    LAST_TS=$(stat -c %Y "$LOCK_FILE" 2>/dev/null || echo 0)
    NOW_TS=$(date +%s)
    if (( NOW_TS - LAST_TS < 60 )); then
        exit 0
    fi
fi

# Increment session counter
CURRENT=$(grep '^sessions_since=' "$STATE_FILE" | cut -d= -f2 || echo 0)
NEW_COUNT=$(( CURRENT + 1 ))
sed -i "s/^sessions_since=.*/sessions_since=${NEW_COUNT}/" "$STATE_FILE"

# Update debounce timestamp
touch "$LOCK_FILE"
