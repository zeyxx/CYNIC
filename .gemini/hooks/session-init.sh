#!/usr/bin/env bash
# CYNIC — Gemini CLI SessionStart hook
# Mirrors Claude's session-init.sh: probe kernel, register agent, inject context.
# KEY DIFFERENCE: stdout MUST be pure JSON (Gemini requirement).
# All human-readable output goes to stderr.
set -euo pipefail

INPUT=$(cat)
PROJECT_DIR="${GEMINI_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"

source ~/.cynic-env 2>/dev/null || true

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"
AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# ── Derive agent ID ──
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || true)
AGENT_ID=""
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="gemini-${SESSION_ID:0:12}"
else
    AGENT_ID="gemini-$(date +%s | tail -c 13)"
fi

# ── Kernel health probe ──
HEALTH_CODE=$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    "http://${KERNEL_ADDR}/health" 2>/dev/null || echo "000")

if [[ "$HEALTH_CODE" == "200" ]]; then
    KERNEL_STATUS="sovereign"
elif [[ "$HEALTH_CODE" == "503" ]]; then
    KERNEL_STATUS="degraded"
else
    KERNEL_STATUS="down"
fi

# ── Dog count ──
ACTIVE_DOGS=0
if [[ "$KERNEL_STATUS" != "down" ]]; then
    ACTIVE_DOGS=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/health" 2>/dev/null \
        | jq '.dogs | length' 2>/dev/null || echo 0)
fi

# ── Register agent ──
if [[ "$KERNEL_STATUS" != "down" ]]; then
    curl -s --max-time 2 -X POST "http://${KERNEL_ADDR}/coord/register" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"intent\":\"gemini session\",\"agent_type\":\"gemini\"}" \
        > /dev/null 2>&1 || true
fi

# ── Git state ──
GIT_BRANCH=$(git -C "$PROJECT_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GIT_DIRTY=$(git -C "$PROJECT_DIR" status --short 2>/dev/null | grep -vc '^??' || echo 0)

# ── Build context string ──
CONTEXT="CYNIC Organism State (Gemini session ${AGENT_ID})
Kernel: ${KERNEL_STATUS} | Dogs: ${ACTIVE_DOGS} | Branch: ${GIT_BRANCH} | Dirty: ${GIT_DIRTY}
Date: $(date '+%Y-%m-%d %H:%M') | Agent: ${AGENT_ID}"

# ── Crystals (top 3 canonical) ──
if [[ "$KERNEL_STATUS" != "down" ]]; then
    CRYSTALS=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/crystals?limit=3" 2>/dev/null \
        | jq -r '.crystals[]? | "Crystal [\(.state)]: \(.content[:120])"' 2>/dev/null || true)
    if [[ -n "$CRYSTALS" ]]; then
        CONTEXT="${CONTEXT}
Top crystals:
${CRYSTALS}"
    fi
fi

# ── Mempool (ripe items) ──
if [[ "$KERNEL_STATUS" != "down" ]]; then
    MEMPOOL=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/observations?domain=mempool&limit=5" 2>/dev/null \
        | jq -r '.[]? | "[\(.tool // "?")] \(.context[:80])"' 2>/dev/null || true)
    if [[ -n "$MEMPOOL" ]]; then
        CONTEXT="${CONTEXT}
Mempool:
${MEMPOOL}"
    fi
fi

# ── Active agents (MC4 collision detection) ──
if [[ "$KERNEL_STATUS" != "down" ]]; then
    AGENTS=$(curl -s --max-time 2 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/coord/who" 2>/dev/null \
        | jq -r '.agents[]? | "\(.agent_id) scope=\(.scope // "none")"' 2>/dev/null || true)
    if [[ -n "$AGENTS" ]]; then
        CONTEXT="${CONTEXT}
Active agents:
${AGENTS}"
    fi
fi

# ── Zone awareness (shared with Claude/Codex via /tmp/cynic-zones/) ──
ZONE_STATE_DIR="/tmp/cynic-zones"
ZONES_FILE="${PROJECT_DIR}/.claude/zones.json"
if [[ -d "$ZONE_STATE_DIR" ]] && ls "$ZONE_STATE_DIR"/*.claimed &>/dev/null; then
    ZONE_INFO="Zone claims (Edit/Write BLOCKED on conflict):"
    for lock in "$ZONE_STATE_DIR"/*.claimed; do
        [[ -f "$lock" ]] || continue
        ZONE_NAME=$(basename "$lock" .claimed)
        CLAIMED_BY=$(cat "$lock" 2>/dev/null || echo "?")
        if [[ "$CLAIMED_BY" == "$AGENT_ID" ]]; then
            ZONE_INFO="${ZONE_INFO}
  ${ZONE_NAME} → ${CLAIMED_BY} (YOU)"
        else
            ZONE_INFO="${ZONE_INFO}
  ${ZONE_NAME} → ${CLAIMED_BY} ← BLOCKED"
        fi
    done
    if [[ -f "$ZONES_FILE" ]]; then
        FREE=$(jq -r '.zones | keys[]' "$ZONES_FILE" 2>/dev/null | while read z; do
            [[ ! -f "$ZONE_STATE_DIR/${z}.claimed" ]] && echo -n "$z "
        done)
        [[ -n "$FREE" ]] && ZONE_INFO="${ZONE_INFO}
  FREE: ${FREE}"
    fi
    CONTEXT="${CONTEXT}
${ZONE_INFO}"
fi

# ── Session state file (for session-stop metrics) ──
SESSION_STATE_DIR="/tmp/cynic-sessions"
mkdir -p "$SESSION_STATE_DIR" 2>/dev/null || true
echo "agent_id=${AGENT_ID}
session_start=$(date +%s)
project_dir=${PROJECT_DIR}
agent_type=gemini" > "${SESSION_STATE_DIR}/${AGENT_ID}.state"

# ── Output: Gemini requires pure JSON on stdout ──
# hookSpecificOutput.additionalContext injects into the first turn
jq -n \
    --arg context "$CONTEXT" \
    --arg msg "CYNIC: kernel=${KERNEL_STATUS}, dogs=${ACTIVE_DOGS}, branch=${GIT_BRANCH}" \
    '{
        hookSpecificOutput: {
            additionalContext: $context
        },
        systemMessage: $msg
    }'

# Log to stderr (visible in terminal but not parsed as JSON)
echo "CYNIC: kernel=${KERNEL_STATUS} dogs=${ACTIVE_DOGS} agent=${AGENT_ID}" >&2
