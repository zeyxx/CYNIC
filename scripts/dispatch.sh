#!/usr/bin/env bash
# CYNIC — Multi-cortex dispatch generator
# Usage: ./scripts/dispatch.sh
# Shows zone map, active claims, and generates structured dispatch prompts.
set -euo pipefail

PROJECT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
ZONES_FILE="${PROJECT_DIR}/.cortex/zones.json"
ZONE_STATE_DIR="/tmp/cynic-zones"

source ~/.cynic-env 2>/dev/null || true
KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

# ── Show current state ──

echo "╔══════════════════════════════════════════╗"
echo "║  CYNIC — Multi-Cortex Dispatch           ║"
echo "╚══════════════════════════════════════════╝"
echo ""

# Active zone claims
echo "ZONE CLAIMS (local):"
if [[ -d "$ZONE_STATE_DIR" ]]; then
    HAS_CLAIMS=false
    for lock in "$ZONE_STATE_DIR"/*.claimed; do
        [[ -f "$lock" ]] || continue
        HAS_CLAIMS=true
        ZONE_NAME=$(basename "$lock" .claimed)
        CLAIMED_BY=$(cat "$lock" 2>/dev/null || echo "?")
        printf "  %-20s → %s\n" "$ZONE_NAME" "$CLAIMED_BY"
    done
    $HAS_CLAIMS || echo "  (none — all zones free)"
else
    echo "  (none — all zones free)"
fi
echo ""

# Active mempool dispatches
echo "ACTIVE DISPATCHES (mempool):"
if [[ -n "${API_KEY:-}" ]]; then
    DISPATCHES=$(curl -s --max-time 3 \
        -H "Authorization: Bearer ${API_KEY}" \
        "http://${KERNEL_ADDR}/observations?domain=mempool&limit=10" 2>/dev/null || echo "[]")
    echo "$DISPATCHES" | jq -r '.[] | select(.tool == "human_dispatch") | "  \(.target // "?") — \(.context // "" | .[0:80])"' 2>/dev/null || echo "  (kernel unreachable)"
    DISPATCH_COUNT=$(echo "$DISPATCHES" | jq '[.[] | select(.tool == "human_dispatch")] | length' 2>/dev/null || echo 0)
    [[ "$DISPATCH_COUNT" == "0" ]] && echo "  (none)"
else
    echo "  (no API key — skipping)"
fi
echo ""

# Zone map
echo "AVAILABLE ZONES:"
if [[ -f "$ZONES_FILE" ]]; then
    jq -r '.zones | to_entries[] | "  \(.key):\t\(.value.description) [\(.value.paths | join(", "))]"' "$ZONES_FILE" | column -t -s $'\t'
fi
echo ""

# ── Generate dispatch ──

echo "────────────────────────────────────────────"
echo "Paste one of these as the FIRST MESSAGE to each cortex:"
echo "────────────────────────────────────────────"
echo ""
echo "Example dispatch for 3 parallel cortex:"
echo ""

cat << 'DISPATCH_A'
═══ CORTEX A (copy-paste this) ═══

SCOPE: api
ZONES: cynic-kernel/src/api/
TASK: [describe the task]
DO NOT TOUCH: cynic-kernel/src/domain/ cynic-kernel/src/storage/ cynic-node/

DISPATCH_A

cat << 'DISPATCH_B'
═══ CORTEX B (copy-paste this) ═══

SCOPE: storage
ZONES: cynic-kernel/src/storage/
TASK: [describe the task]
DO NOT TOUCH: cynic-kernel/src/api/ cynic-kernel/src/domain/ cynic-node/

DISPATCH_B

cat << 'DISPATCH_C'
═══ CORTEX C (copy-paste this) ═══

SCOPE: docs + hooks
ZONES: docs/ .cortex/mcp/ .cortex/rules/
TASK: [describe the task]
DO NOT TOUCH: cynic-kernel/

DISPATCH_C

echo "────────────────────────────────────────────"
echo "RULES:"
echo "  1. domain-core zone: max 1 cortex (collision hub)"
echo "  2. Zone claims are HARD gates — Edit/Write blocked on conflict"
echo "  3. Claims release on session end (session-stop.sh)"
echo "  4. Bridge posts dispatch to mempool — next cortex sees it"
echo "  5. Override: CYNIC_COORD_ALLOW_DEGRADED=1 (emergency only)"
