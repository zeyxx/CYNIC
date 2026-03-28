#!/usr/bin/env bash
# CYNIC — SessionEnd hook (dream trigger)
# Records session end, increments counter.
# SessionStart (session-init.sh) checks conditions and suggests /dream.
# Mechanical — no LLM involved.
set -euo pipefail

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
MEMORY_DIR="${HOME}/.claude/projects/-home-user-Bureau-CYNIC/memory"
STATE_FILE="${MEMORY_DIR}/.dream-state"

# Initialize state file if missing
if [[ ! -f "$STATE_FILE" ]]; then
    cat > "$STATE_FILE" <<EOF
last_dream=$(date -Iseconds)
sessions_since=0
EOF
fi

# Increment session counter (atomic: read → increment → write)
CURRENT=$(grep '^sessions_since=' "$STATE_FILE" | cut -d= -f2 || echo 0)
NEW_COUNT=$(( CURRENT + 1 ))
sed -i "s/^sessions_since=.*/sessions_since=${NEW_COUNT}/" "$STATE_FILE"
