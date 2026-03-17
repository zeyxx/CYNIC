#!/usr/bin/env bash
# CYNIC — Stop hook
# Clean shutdown: release coord claims, audit session-end, deactivate agent.
# Mirrors session-init.sh — together they bookend the session lifecycle.
#
# Uses SurrealDB HTTP directly (coord is MCP-only, not REST).
# Single multi-statement request to avoid timeout risk.
set -euo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

# Agent ID from Claude session_id (same derivation as session-init.sh)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    exit 0
fi

# SurrealDB direct access
SURREAL_URL="${SURREALDB_URL:-http://localhost:8000}"
SURREAL_USER="${SURREALDB_USER:-root}"
SURREAL_PASS="${SURREALDB_PASS:-}"

if [[ -z "$SURREAL_PASS" ]]; then
    exit 0
fi

AUTH=$(echo -n "${SURREAL_USER}:${SURREAL_PASS}" | base64)
# Escape single quotes for SurrealQL (backslash-quote, matching escape_surreal() in Rust)
SAFE_ID=$(echo "$AGENT_ID" | sed "s/'/\\\\'/g")

# All 3 operations in one HTTP request — no sequential timeout risk
curl -s --max-time 5 -X POST "${SURREAL_URL}/sql" \
    -H "Accept: application/json" \
    -H "surreal-ns: cynic" -H "surreal-db: v2" \
    -H "Authorization: Basic ${AUTH}" \
    -d "UPDATE work_claim SET active = false WHERE agent_id = '${SAFE_ID}' AND active = true; \
        UPDATE agent_session SET active = false WHERE agent_id = '${SAFE_ID}'; \
        CREATE mcp_audit SET ts = time::now(), tool = 'session_end', agent_id = '${SAFE_ID}', details = 'clean shutdown';" \
    > /dev/null 2>&1 || true
