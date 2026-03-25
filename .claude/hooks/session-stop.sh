#!/usr/bin/env bash
# CYNIC — Stop hook
# Clean shutdown: release all coord claims via REST API.
# Mirrors session-init.sh — together they bookend the session lifecycle.
#
# Uses kernel REST API (POST /coord/release) — NOT SurrealDB direct.
# The kernel routes through CoordPort → SurrealHttpStorage.
set -euo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

# Agent ID from Claude session_id (same derivation as session-init.sh)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || true)
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    # Fallback: try timestamp-based ID (mirrors session-init.sh fallback)
    # Without a stable ID we can't release — but expire_stale will clean up in 5min
    exit 0
fi

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# Release ALL claims for this agent (no target = release all)
curl -s --connect-timeout 2 --max-time 5 -X POST "http://${KERNEL_ADDR}/coord/release" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    -d "{\"agent_id\":\"${AGENT_ID}\"}" \
    > /dev/null 2>&1 || true

# ── Rule #30: warn about uncommitted changes (staged + unstaged + untracked) ──
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
DIRTY=$(git -C "$PROJECT_DIR" status --short 2>/dev/null | grep -v '^??' | head -5 || true)
if [[ -n "$DIRTY" ]]; then
    echo "WARNING: Uncommitted changes (Rule #30):"
    echo "$DIRTY"
fi
