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
AGENT_ID=""

if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    # Fallback: read from most recent session state file (set by session-init.sh)
    SESSION_STATE_DIR="/tmp/cynic-sessions"
    if [[ -d "$SESSION_STATE_DIR" ]]; then
        RECENT_STATE=$(ls -t "$SESSION_STATE_DIR"/*.state 2>/dev/null | head -1)
        if [[ -n "$RECENT_STATE" ]]; then
            AGENT_ID=$(grep -oP 'agent_id=\K[^ ]+' "$RECENT_STATE" 2>/dev/null || true)
        fi
    fi
fi

if [[ -z "$AGENT_ID" ]]; then
    # Without a stable ID we can't release — kernel's expire_stale will clean up in 5min
    echo "Warning: could not determine AGENT_ID for coordination release" >&2
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

# ── Temporal compliance measurement (Kairos consciousness) ──
# Measures whether the mempool was used during this session.
# K15 consumer: compound loop accumulates these for maturity model calibration.
TEMPORAL_COMPLIANCE="unknown"
MEMPOOL_ACTIVITY=0
KERNEL_CHECK=$(curl -s --connect-timeout 2 --max-time 3 -o /dev/null -w '%{http_code}' \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    "http://${KERNEL_ADDR}/health" 2>/dev/null) || KERNEL_CHECK="000"
if [[ "$KERNEL_CHECK" =~ ^(200|503)$ ]]; then
    MEMPOOL_ACTIVITY=$(curl -s --connect-timeout 2 --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/observations?domain=mempool&limit=20" 2>/dev/null \
        | jq "[.[]? | select(.agent_id == \"${AGENT_ID}\")] | length" 2>/dev/null || echo 0)

    if [[ "$MEMPOOL_ACTIVITY" -gt 0 ]]; then
        TEMPORAL_COMPLIANCE="active"
    else
        TEMPORAL_COMPLIANCE="inactive"
    fi

    # Store temporal metrics for compound loop
    curl -s --connect-timeout 2 --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"tool\":\"session_temporal\",\"target\":\"compliance\",\"domain\":\"temporal-meta\",\"context\":\"mempool_activity=${MEMPOOL_ACTIVITY} compliance=${TEMPORAL_COMPLIANCE}\",\"tags\":[\"temporal-meta\"]}" \
        > /dev/null 2>&1 || true
fi
echo "Temporal: ${TEMPORAL_COMPLIANCE} (${MEMPOOL_ACTIVITY} mempool observations)"

# ── K15: Update AT_END proof in .claude/session-proof.json (immutable record) ──
# Read the AT_START proof that session-init.sh created, append AT_END fields.
# This creates a complete proof record for next session to audit.
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
PROOF_FILE="${PROJECT_DIR}/.claude/session-proof.json"

if [[ -f "$PROOF_FILE" ]]; then
    # Add AT_END fields to existing proof (uses jq to merge safely)
    END_COMMIT=$(git -C "$PROJECT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")
    BRANCHES_DELETED=$(git -C "$PROJECT_DIR" branch -vv 2>/dev/null | grep -c '\[gone\]' || true)

    # Tentative: has any staged/unstaged changes? (Would be lost if not committed)
    WORK_LOST=""
    DIRTY=$(git -C "$PROJECT_DIR" status --porcelain 2>/dev/null | grep -v '^??' | head -3 || true)
    if [[ -n "$DIRTY" ]]; then
        WORK_LOST="uncommitted changes present (review git status)"
    fi

    # Use jq to append AT_END to the proof JSON (safe merge)
    jq ".AT_END = {
      \"final_commit_hash\": \"${END_COMMIT}\",
      \"branches_deleted\": ${BRANCHES_DELETED},
      \"duration_minutes\": ${SESSION_MINUTES:-0},
      \"commits_produced\": ${COMMITS_THIS_SESSION:-0},
      \"compliance_score\": ${SCORE:-\"unknown\"},
      \"work_lost\": $(jq -R -s -c . <<< "${WORK_LOST:-none}"),
      \"completed_at\": \"$(date -u +'%Y-%m-%dT%H:%M:%SZ')\"
    }" "$PROOF_FILE" > "${PROOF_FILE}.tmp" && mv "${PROOF_FILE}.tmp" "$PROOF_FILE"
fi

# ── Inter-agent bus: POST session summary to /observe (domain=session) ──
# This is the structured handover channel. Other agents (Claude, Gemini, nightshift)
# read domain="session" observations at session start to understand prior work.
# K15: Proof-of-History schema — includes cortex proof (5 fields) + session metadata
KERNEL_STATUS=$(curl -s --connect-timeout 2 --max-time 3 "http://${KERNEL_ADDR}/health" 2>/dev/null | jq -r '.status // "down"' 2>/dev/null || echo "down")
if [[ "$KERNEL_STATUS" != "down" ]]; then
    # Collect session metadata
    COMMITS_COUNT="${COMMITS_THIS_SESSION:-0}"
    DURATION_MIN="${SESSION_MINUTES:-0}"
    COMPLIANCE_SCORE="${SCORE:-unknown}"

    # K15: Cortex proof (5-field minimal schema for multi-cortex coordination)
    PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
    BRANCH_NAME=$(git -C "$PROJECT_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
    BASE_SHA=$(git -C "$PROJECT_DIR" merge-base HEAD origin/main 2>/dev/null | cut -c1-7 || echo "unknown")
    HEAD_SHA=$(git -C "$PROJECT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")
    GATES_PASSED="false"
    # Check gates: compliance > 0.5 = gates passed (awk avoids bc dependency)
    if [[ "${COMPLIANCE_SCORE}" != "unknown" ]]; then
        GATES_PASSED=$(awk "BEGIN {print (${COMPLIANCE_SCORE} > 0.5) ? \"true\" : \"false\"}" 2>/dev/null || echo "false")
    fi
    PROOF_TIMESTAMP=$(date -u +'%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || echo "unknown")

    # Build a structured summary (context field, 2000 char max in kernel)
    # Format: key=value pairs, space-separated
    SUMMARY="dur=${DURATION_MIN}m commits=${COMMITS_COUNT} compliance=${COMPLIANCE_SCORE} branch=${BRANCH_NAME} base_sha=${BASE_SHA} head_sha=${HEAD_SHA} gates=${GATES_PASSED} timestamp=${PROOF_TIMESTAMP}"

    curl -s --connect-timeout 2 --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"tool\":\"session_summary\",\"target\":\"handover\",\"domain\":\"session\",\"tags\":[\"session-summary\",\"cortex-proof\"],\"context\":\"${SUMMARY}\"}" \
        > /dev/null 2>&1 || true
fi

# ── K15: Git ledger — link commits to observation chain (blockchain prism) ──
# Records HEAD commit hash and parent for hash-chain linkage.
# Next session can detect if HEAD moved (cortex did work) or was reset (rebase).
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
CURRENT_HASH=$(git -C "$PROJECT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")
PARENT_HASH=$(git -C "$PROJECT_DIR" rev-parse --short HEAD^ 2>/dev/null || echo "root")

if [[ "$KERNEL_STATUS" != "down" ]] && [[ "$CURRENT_HASH" != "unknown" ]]; then
    # List files changed this session (for ledger audit trail)
    FILES_CHANGED=$(git -C "$PROJECT_DIR" diff --name-only origin/main...HEAD 2>/dev/null | jq -R -s -c 'split("\n")[:-1]' 2>/dev/null || echo '[]')

    # POST to kernel ledger — domain=git enables multi-cortex divergence detection
    curl -s --connect-timeout 2 --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"tool\":\"session_git_proof\",\"target\":\"${BRANCH_NAME:-HEAD}\",\"domain\":\"git\",\"hash\":\"${CURRENT_HASH}\",\"prev_hash\":\"${PARENT_HASH}\",\"consumer\":\"coord\",\"action\":\"git state link\",\"tags\":[\"git-ledger\"],\"context\":\"files=${FILES_CHANGED}\"}" \
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

# ── Distill enforcement (K15: session reasoning must reach the organism) ──
if [[ "$KERNEL_STATUS" != "down" ]]; then
    DISTILL_CHECK=$(curl -s --connect-timeout 2 --max-time 4 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/observations?agent_id=${AGENT_ID}&domain=session&limit=20" \
        2>/dev/null || echo "[]")
    DISTILL_FOUND=$(echo "$DISTILL_CHECK" | jq -r \
        '[.[] | select(.tool == "session_distill")] | length' 2>/dev/null || echo "0")
    if [[ "$DISTILL_FOUND" == "0" ]]; then
        echo ""
        echo "DISTILL MISSING — organism cannot learn from this session."
        echo "Run before closing:"
        echo "  source ~/.cynic-env && curl -s -X POST \"\${CYNIC_REST_ADDR}/observe\" \\"
        echo "    -H \"Authorization: Bearer \${CYNIC_API_KEY}\" \\"
        echo "    -H \"Content-Type: application/json\" \\"
        echo "    -d '{\"tool\":\"session_distill\",\"target\":\"handover\",\"domain\":\"session\",\"agent_id\":\"${AGENT_ID}\",\"tags\":[\"session-distill\"],\"context\":\"WHAT: <done> WHY: <decisions> NEXT: <next> BLOCKED: <stuck>\"}'"
    fi
fi
