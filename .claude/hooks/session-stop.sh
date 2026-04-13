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

# ── Session compliance score (Phase 2: process loop) ──
# Non-blocking: if kernel is down, skip gracefully.
COMPLIANCE=$(curl -s --connect-timeout 2 --max-time 5 \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    "http://${KERNEL_ADDR}/session/${AGENT_ID}/compliance" 2>/dev/null || echo "")
if [[ -n "$COMPLIANCE" ]] && echo "$COMPLIANCE" | jq -e '.score' > /dev/null 2>&1; then
    SCORE=$(echo "$COMPLIANCE" | jq -r '.score' 2>/dev/null)
    WARNINGS=$(echo "$COMPLIANCE" | jq -r '.warnings[]' 2>/dev/null || true)
    RBE=$(echo "$COMPLIANCE" | jq -r '.read_before_edit' 2>/dev/null)
    FM=$(echo "$COMPLIANCE" | jq -r '.files_modified' 2>/dev/null)
    echo ""
    # Force C locale for printf — French locale uses commas for decimals
    RBE_PCT=$(LC_ALL=C awk "BEGIN {printf \"%.0f\", $RBE * 100}" 2>/dev/null || echo "?")
    LC_ALL=C printf "Session compliance: %.3f/0.618  (read-before-edit: %s%%, files: %s)\n" "$SCORE" "$RBE_PCT" "$FM"
    if [[ -n "$WARNINGS" ]]; then
        while IFS= read -r W; do
            echo "  ⚠ $W"
        done <<< "$WARNINGS"
    fi
fi

# ── K15: compliance score → acting consumer (POST to /observe) ──
if [[ -n "$COMPLIANCE" ]] && echo "$COMPLIANCE" | jq -e '.score' > /dev/null 2>&1; then
    curl -s --connect-timeout 2 --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"tool\":\"session_compliance\",\"target\":\"session_end\",\"domain\":\"general\",\"context\":\"score=${SCORE} rbe=${RBE_PCT}% files=${FM}\"}" \
        > /dev/null 2>&1 || true
fi

# ── Rule 3: Session cost tracking (duration, commits, context size) ──
SESSION_STATE_DIR="/tmp/cynic-sessions"
SESSION_STATE_FILE="${SESSION_STATE_DIR}/${AGENT_ID}.state"
SESSION_COST=""
if [[ -f "$SESSION_STATE_FILE" ]]; then
    source "$SESSION_STATE_FILE" 2>/dev/null || true
    if [[ -n "${session_start:-}" ]]; then
        SESSION_END=$(date +%s)
        SESSION_DURATION=$((SESSION_END - session_start))
        SESSION_MINUTES=$((SESSION_DURATION / 60))

        # Commit count (via git log)
        PROJECT_DIR="${project_dir:-.}"
        COMMITS_THIS_SESSION=$(git -C "$PROJECT_DIR" rev-list --since="${session_start}" --until="${SESSION_END}" --count HEAD 2>/dev/null || echo 0)

        # Context: estimate from git object count delta (rough signal)
        # Real token usage would need Claude Code API integration (future)
        SESSION_COST="duration_sec=${SESSION_DURATION} duration_min=${SESSION_MINUTES} commits=${COMMITS_THIS_SESSION}"

        # POST cost metrics to /observe (K15 consumer)
        curl -s --connect-timeout 2 --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
            -H "Content-Type: application/json" \
            ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
            -d "{\"agent_id\":\"${AGENT_ID}\",\"tool\":\"session_cost\",\"target\":\"session_end\",\"domain\":\"general\",\"context\":\"${SESSION_COST}\"}" \
            > /dev/null 2>&1 || true

        echo "Session cost: ${SESSION_MINUTES}min, ${COMMITS_THIS_SESSION} commits"
        rm -f "$SESSION_STATE_FILE"  # Clean up after measuring
    fi
fi

# ── Inter-agent bus: POST session summary to /observe (domain=session) ──
# This is the structured handover channel. Other agents (Claude, Gemini, nightshift)
# read domain="session" observations at session start to understand prior work.
if [[ "$KERNEL_STATUS" != "down" ]]; then
    # Collect session metadata
    COMMITS_COUNT="${COMMITS_THIS_SESSION:-0}"
    DURATION_MIN="${SESSION_MINUTES:-0}"
    COMPLIANCE_SCORE="${SCORE:-unknown}"

    # Build a structured summary (context field, 200 char max enforced by kernel)
    SUMMARY="dur=${DURATION_MIN}m commits=${COMMITS_COUNT} compliance=${COMPLIANCE_SCORE}"

    curl -s --connect-timeout 2 --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"tool\":\"session_summary\",\"target\":\"handover\",\"domain\":\"session\",\"tags\":[\"session-summary\"],\"context\":\"${SUMMARY}\"}" \
        > /dev/null 2>&1 || true
fi

# ── TODO staleness check (continuity: did this session update the TODO?) ──
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
TODO_FILE="${PROJECT_DIR}/TODO.md"
if [[ -f "$TODO_FILE" ]]; then
    TODO_CHANGED=$(git -C "$PROJECT_DIR" diff --name-only -- TODO.md 2>/dev/null || true)
    if [[ -z "$TODO_CHANGED" ]]; then
        echo "NOTE: TODO.md not updated this session. Review if any items were completed or discovered."
    fi
fi

# ── Rule 4: warn about uncommitted changes (staged + unstaged + untracked) ──
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
DIRTY=$(git -C "$PROJECT_DIR" status --short 2>/dev/null | grep -v '^??' | head -5 || true)
if [[ -n "$DIRTY" ]]; then
    echo "WARNING: Uncommitted changes (Rule 4):"
    echo "$DIRTY"
fi
