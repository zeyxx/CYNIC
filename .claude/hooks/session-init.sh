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
KERNEL_STATUS="down"
if curl -s --max-time 2 "http://${CYNIC_REST_ADDR:-localhost:3030}/health" | jq -r '.status' > /dev/null 2>&1; then
    KERNEL_STATUS=$(curl -s --max-time 2 "http://${CYNIC_REST_ADDR}/health" | jq -r '.status')
fi

SURREAL_STATUS="down"
if surreal is-ready --endpoint http://localhost:8000 2>/dev/null; then
    SURREAL_STATUS="ok"
fi

GIT_BRANCH=$(git -C "$PROJECT_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GIT_DIRTY=$(git -C "$PROJECT_DIR" status --porcelain 2>/dev/null | wc -l)

# ── Output context (injected into conversation) ──
cat <<EOF
CYNIC SESSION — Pipeline initialized.
Kernel: ${KERNEL_STATUS} | DB: ${SURREAL_STATUS} | Git: ${GIT_BRANCH} (${GIT_DIRTY} dirty files)
Env: CYNIC_REST_ADDR=${CYNIC_REST_ADDR:-NOT SET}

WORKFLOW: Use /build after edits, /deploy for production, /status for full dashboard.
RULES: Public repo — no secrets, no real IPs, no names. Use skills before acting.
EOF
