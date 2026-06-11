#!/usr/bin/env bash
# CYNIC — Shared Session Stop Hook
# Automates governance, coordination release, and metrics extraction.
# Shared between Claude and Gemini.
set -euo pipefail

# Support both Claude and Gemini input formats
INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

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

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"
AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# 1. Release all coordination claims
echo "Releasing claims for ${AGENT_ID}..." >&2
curl -s --connect-timeout 2 --max-time 5 -X POST "http://${KERNEL_ADDR}/coord/release" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    -d "{\"agent_id\":\"${AGENT_ID}\"}" \
    > /dev/null 2>&1 || true

# 2. Check TODO.md for phase transitions (Governance Continuity)
PROJECT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
TODO_FILE="${PROJECT_DIR}/TODO.md"

if [[ -f "$TODO_FILE" ]]; then
    # Extract active phase and next phase
    # This logic assumes the structure:
    # ## NOW (YYYY-MM-DD)
    # **Phase X: Name**
    # ...
    # ## IMMEDIATE (YYYY-MM-DD)
    # **Phase Y: Name**
    
    NOW_SECTION=$(grep -A 20 "^## NOW" "$TODO_FILE" 2>/dev/null || true)
    ACTIVE_PHASE=$(echo "$NOW_SECTION" | grep "^**Phase" | head -1 | sed 's/.*\*\*Phase \([^:]*\).*/\1/' 2>/dev/null || true)
    
    IMMEDIATE_SECTION=$(grep -A 30 "^## IMMEDIATE" "$TODO_FILE" 2>/dev/null || true)
    NEXT_PHASE=$(echo "$IMMEDIATE_SECTION" | grep "^**Phase" | head -1 | sed 's/.*\*\*Phase \([^:]*\).*/\1/' 2>/dev/null || true)
    
    if [[ -n "$ACTIVE_PHASE" && -n "$NEXT_PHASE" ]]; then
        # Check if active phase tasks are all completed
        # Simple heuristic: are there any unchecked boxes in the NOW section's action items?
        PENDING_TASKS=$(echo "$NOW_SECTION" | grep -c "\- \[ \]" || true)
        
        if [[ "$PENDING_TASKS" -eq 0 ]]; then
            echo "Governance: Phase ${ACTIVE_PHASE} appears complete. Queuing Phase ${NEXT_PHASE}..." >&2
            
            # Post to mempool (observations domain)
            curl -s --connect-timeout 2 --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
                -H "Content-Type: application/json" \
                ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
                -d "{
                    \"agent_id\":\"${AGENT_ID}\",
                    \"tool\":\"governance\",
                    \"target\":\"phase_transition\",
                    \"domain\":\"mempool\",
                    \"tags\":[\"governance\", \"phase-transition\"],
                    \"context\":\"Phase ${ACTIVE_PHASE} is COMPLETE. Next up: Phase ${NEXT_PHASE}. Action: Move Phase ${NEXT_PHASE} to ## NOW in TODO.md and start execution.\"
                }" > /dev/null 2>&1 || true
        else
            echo "Governance: Phase ${ACTIVE_PHASE} still has ${PENDING_TASKS} pending tasks." >&2
        fi
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
