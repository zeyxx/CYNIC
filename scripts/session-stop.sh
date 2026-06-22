#!/usr/bin/env bash
# CYNIC — Shared Session Stop Hook
# Automates governance, coordination release, and metrics extraction.
# Shared between Claude and Gemini.
set -euo pipefail

# Support both Claude and Gemini input formats
INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/cynic-endpoint.sh"
if ! KERNEL_ADDR="$(cynic_kernel_rest_url 2>/dev/null)"; then
    KERNEL_ADDR=""
fi

# Determine Agent ID
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || true)
AGENT_ID=""

if [[ -n "$SESSION_ID" ]]; then
    # Claude-style or Gemini-style with session_id
    AGENT_ID="cortex-${SESSION_ID:0:12}"
else
    # Fallback to .cortex-session if it exists
    PROJECT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
    if [[ -f "$PROJECT_DIR/.cortex-session" ]]; then
        AGENT_ID="cortex-$(cat "$PROJECT_DIR/.cortex-session" | tr -d '[:space:]')"
    fi
fi

if [[ -z "$AGENT_ID" ]]; then
    AGENT_ID="cortex-unknown"
fi

API_KEY="${CYNIC_API_KEY:-}"
AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# 1. Release all coordination claims
echo "Releasing claims for ${AGENT_ID}..." >&2
if [[ -n "${KERNEL_ADDR}" ]]; then
    curl -s --connect-timeout 2 --max-time 5 -X POST "${KERNEL_ADDR}/coord/release" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\"}" \
        > /dev/null 2>&1 || true
else
    echo "Kernel endpoint unavailable; claim release skipped." >&2
fi

# 2. Check governance phase state JSON for continuity (non-markdown)
PROJECT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
GOV_STATE_FILE="${PROJECT_DIR}/.cynic/governance-state.json"

ACTIVE_PHASE=""
NEXT_PHASE=""
ACTIVE_PHASE_PENDING_TASKS=""
ACTIVE_PHASE_COMPLETE=""

# Prefer explicit phase state from governance JSON
if [[ -f "$GOV_STATE_FILE" ]]; then
    ACTIVE_PHASE=$(jq -r '.active_phase // empty' "$GOV_STATE_FILE" 2>/dev/null || true)
    NEXT_PHASE=$(jq -r '.next_phase // empty' "$GOV_STATE_FILE" 2>/dev/null || true)
    ACTIVE_PHASE_PENDING_TASKS=$(jq -r '.active_phase_pending_tasks // empty' "$GOV_STATE_FILE" 2>/dev/null || true)
    ACTIVE_PHASE_COMPLETE=$(jq -r '.active_phase_complete // empty' "$GOV_STATE_FILE" 2>/dev/null || true)
fi

# Fallback: read from hook input JSON if session runtime provides state
if [[ -z "$ACTIVE_PHASE" ]]; then
    ACTIVE_PHASE=$(echo "$INPUT" | jq -r '.active_phase // empty' 2>/dev/null || true)
fi
if [[ -z "$NEXT_PHASE" ]]; then
    NEXT_PHASE=$(echo "$INPUT" | jq -r '.next_phase // empty' 2>/dev/null || true)
fi
if [[ -z "$ACTIVE_PHASE_PENDING_TASKS" ]]; then
    ACTIVE_PHASE_PENDING_TASKS=$(echo "$INPUT" | jq -r '.active_phase_pending_tasks // empty' 2>/dev/null || true)
fi
if [[ -z "$ACTIVE_PHASE_COMPLETE" ]]; then
    ACTIVE_PHASE_COMPLETE=$(echo "$INPUT" | jq -r '.active_phase_complete // empty' 2>/dev/null || true)
fi

if [[ -n "$ACTIVE_PHASE" && -n "$NEXT_PHASE" ]]; then
    if [[ "$ACTIVE_PHASE_COMPLETE" == "true" ]]; then
        PENDING_TASKS=0
    else
        PENDING_TASKS="${ACTIVE_PHASE_PENDING_TASKS:-0}"
    fi

    # Post explicit transition suggestion only when transition is signaled.
    if [[ "$PENDING_TASKS" -eq 0 ]]; then
        echo "Governance: Phase ${ACTIVE_PHASE} appears complete. Queuing Phase ${NEXT_PHASE}..." >&2

        # Post to mempool (observations domain)
        if [[ -n "${KERNEL_ADDR}" ]]; then
            curl -s --connect-timeout 2 --max-time 3 -X POST "${KERNEL_ADDR}/observe" \
                -H "Content-Type: application/json" \
                ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
                -d "{
                    \"agent_id\":\"${AGENT_ID}\",
                    \"tool\":\"governance\",
                    \"target\":\"phase_transition\",
                    \"domain\":\"mempool\",
                    \"tags\":[\"governance\", \"phase-transition\"],
                    \"context\":\"Phase ${ACTIVE_PHASE} is COMPLETE. Next up: Phase ${NEXT_PHASE}. Action: advance active_phase to ${NEXT_PHASE} in governance-state.json and start execution.\"
                }" > /dev/null 2>&1 || true
        else
            echo "Kernel endpoint unavailable; phase-transition observation skipped." >&2
        fi
    else
        echo "Governance: Phase ${ACTIVE_PHASE} still has ${PENDING_TASKS} pending tasks." >&2
    fi
fi

# 3. Warn about uncommitted changes
DIRTY=$(git -C "$PROJECT_DIR" status --short 2>/dev/null | grep -v '^??' | head -5 || true)
if [[ -n "$DIRTY" ]]; then
    echo "WARNING: Uncommitted changes in workspace (Rule 4):" >&2
    echo "$DIRTY" >&2
fi

# 4. Askesis sealing (if applicable)
# For Gemini: ingestion is handled by askesis-session-end.sh which should call this.
# For Claude: ingestion is already handled in .cortex/mcp/session-stop.sh.

echo "{\"status\": \"success\", \"agent_id\": \"${AGENT_ID}\"}"
