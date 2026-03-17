#!/usr/bin/env bash
# CYNIC — SessionStart hook
# Verifies environment and injects critical context at session start and after compaction.
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

# ── Quick health probe ──
HEALTH_JSON=$(curl -s --max-time 2 "http://${CYNIC_REST_ADDR:-localhost:3030}/health" 2>/dev/null || echo '{}')
KERNEL_STATUS=$(echo "$HEALTH_JSON" | jq -r '.status // empty' 2>/dev/null)
[[ -z "$KERNEL_STATUS" ]] && KERNEL_STATUS="down"

# ── Dog drift detection ──
# Count configured backends + deterministic-dog
EXPECTED_DOGS=0
BACKENDS_FILE="${HOME}/.config/cynic/backends.toml"
if [[ -f "$BACKENDS_FILE" ]]; then
    EXPECTED_DOGS=$(( $(grep -c '^\[backend\.' "$BACKENDS_FILE") + 1 ))  # +1 for deterministic-dog
fi
ACTIVE_DOGS=$(echo "$HEALTH_JSON" | jq '.dog_count // 0' 2>/dev/null)
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
# session_id is provided in hook stdin JSON — no temp files needed.
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    # Fallback: timestamp-based (should not happen in practice)
    AGENT_ID="claude-$(date +%s)"
fi

# Mask real IP — session context must never contain real IPs
[ -n "${CYNIC_REST_ADDR:-}" ] && ADDR_STATUS="SET" || ADDR_STATUS="NOT SET"

# ── Output context (injected into conversation) ──
cat <<EOF
CYNIC SESSION — Pipeline initialized.
Kernel: ${KERNEL_STATUS} | DB: ${SURREAL_STATUS} | Git: ${GIT_BRANCH} (${GIT_DIRTY} dirty files)
Dogs: ${ACTIVE_DOGS}/${EXPECTED_DOGS}${DOG_DRIFT:+ — $DOG_DRIFT}
Env: CYNIC_REST_ADDR=${ADDR_STATUS}
Agent: ${AGENT_ID}

WORKFLOW: Use /build after edits, /deploy for production, /status for full dashboard.
COORD: Register → cynic_coord_register("${AGENT_ID}", intent) | Claim → cynic_coord_who + cynic_coord_claim | Release → cynic_coord_release
RULES: Public repo — no secrets, no real IPs, no names. Use skills before acting.
EOF
