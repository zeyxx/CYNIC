#!/usr/bin/env bash
# CYNIC — SessionStart hook
# Verifies environment, auto-registers agent, injects critical context.
# This is the pipeline entry point — every session starts clean.
set -euo pipefail

INPUT=$(cat)
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"

# Source env for variable resolution
source ~/.cynic-env 2>/dev/null || true

# ── Verify working directory ──
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')
if [[ -n "$CWD" && "$CWD" != "$PROJECT_DIR" ]]; then
    echo "WARNING: CWD is $CWD — expected $PROJECT_DIR" >&2
fi

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# ── Quick health probe (authenticated — HTTP 200=sovereign, 503=degraded) ──
HEALTH_CODE=$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    "http://${KERNEL_ADDR}/health" 2>/dev/null || echo "000")

if [[ "$HEALTH_CODE" == "200" ]]; then
    KERNEL_STATUS="sovereign"
elif [[ "$HEALTH_CODE" == "503" ]]; then
    KERNEL_STATUS="degraded"
else
    KERNEL_STATUS="down"
fi

# ── Dog count from authenticated response ──
EXPECTED_DOGS=0
BACKENDS_FILE="${HOME}/.config/cynic/backends.toml"
if [[ -f "$BACKENDS_FILE" ]]; then
    EXPECTED_DOGS=$(( $(grep -c '^\[backend\.' "$BACKENDS_FILE" || echo 0) + 1 ))
fi
# Count dogs from authenticated /health (dogs array length)
ACTIVE_DOGS=0
if [[ "$KERNEL_STATUS" != "down" ]]; then
    ACTIVE_DOGS=$(curl -s --max-time 5 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/health" 2>/dev/null \
        | jq '.dogs | length' 2>/dev/null || echo 0)
fi
DOG_DRIFT=""
if [[ "$EXPECTED_DOGS" -gt 0 && "$ACTIVE_DOGS" -lt "$EXPECTED_DOGS" ]]; then
    DOG_DRIFT="WARNING: ${ACTIVE_DOGS}/${EXPECTED_DOGS} Dogs active — check backend health"
fi

SURREAL_STATUS="down"
if surreal is-ready --endpoint http://localhost:8000 2>/dev/null; then
    SURREAL_STATUS="ok"
fi

GIT_BRANCH=$(git -C "$PROJECT_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GIT_DIRTY=$(git -C "$PROJECT_DIR" status --porcelain 2>/dev/null | wc -l)

# ── Agent ID from Claude session_id (stable across compactions) ──
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    AGENT_ID="claude-$(date +%s)"
fi

# ── Auto-register agent via REST (hard enforcement, not hope) ──
REGISTER_STATUS="skipped"
if [[ "$KERNEL_STATUS" != "down" ]]; then
    REGISTER_RESPONSE=$(curl -s --max-time 3 -X POST "http://${KERNEL_ADDR}/coord/register" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"intent\":\"claude-code session\",\"agent_type\":\"claude\"}" \
        2>/dev/null || echo '{}')
    if echo "$REGISTER_RESPONSE" | jq -e '.status == "registered"' > /dev/null 2>&1; then
        REGISTER_STATUS="registered"
    else
        REGISTER_STATUS="failed"
    fi
fi

# Mask real IP — session context must never contain real IPs
[ -n "${CYNIC_REST_ADDR:-}" ] && ADDR_STATUS="SET" || ADDR_STATUS="NOT SET"

# ── Output context (injected into conversation) ──
cat <<EOF
CYNIC SESSION — Pipeline initialized.
Kernel: ${KERNEL_STATUS} | DB: ${SURREAL_STATUS} | Git: ${GIT_BRANCH} (${GIT_DIRTY} dirty files)
Dogs: ${ACTIVE_DOGS}/${EXPECTED_DOGS}${DOG_DRIFT:+ — $DOG_DRIFT}
Env: CYNIC_REST_ADDR=${ADDR_STATUS}
Agent: ${AGENT_ID} (${REGISTER_STATUS})

WORKFLOW: Use /build after edits, /deploy for production, /status for full dashboard.
COORD: Agent auto-registered. Claim → cynic_coord_who + cynic_coord_claim | Release → cynic_coord_release
RULES: Public repo — no secrets, no real IPs, no names. Use skills before acting.
EOF

# ── Inject top CCM crystals as learnings (CYNIC remembers) ──
if [[ "$KERNEL_STATUS" != "down" ]]; then
    CRYSTALS=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/crystals?limit=5" 2>/dev/null)
    if [[ -n "$CRYSTALS" && "$CRYSTALS" != "[]" ]]; then
        echo ""
        echo "CYNIC MEMORY (top crystallized patterns):"
        echo "$CRYSTALS" | jq -r '.[] | select(.state == "crystallized" or .state == "canonical") | "  [\(.state)] \(.content) (confidence: \(.confidence | tostring | .[0:4]), \(.observations) obs)"' 2>/dev/null | head -5
    fi
fi
