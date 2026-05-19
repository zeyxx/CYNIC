#!/usr/bin/env bash
# CYNIC Dog Health Monitor — K15 Acting Consumer
# Polls kernel /health contract. If contract unfulfilled, restarts local backends.
# Runs every 30s via systemd timer.
#
# K15: Sensor (kernel /health) must have acting consumer.
# Consumer: Auto-restart sovereign backends on contract breach.
# Outcome: Self-heal common failures without human intervention.
#
# The kernel's contract.expected_dogs is the SSOT — zero hardcoded dog names here.

set -euo pipefail

source ~/.cynic-env 2>/dev/null || {
    echo "ERROR: ~/.cynic-env not found"
    exit 1
}

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"
AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# ── Probe kernel health ──
HEALTH=$(timeout 5 curl -s --max-time 5 \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    "http://${KERNEL_ADDR}/health" 2>/dev/null || echo '{}')

# Read contract from kernel (SSOT — no hardcoded dog list)
FULFILLED=$(echo "$HEALTH" | jq -r '.contract.fulfilled // "null"' 2>/dev/null || echo "null")

if [[ "$FULFILLED" == "null" ]]; then
    echo "$(date): Kernel unreachable or contract missing — skipping"
    exit 0
fi

if [[ "$FULFILLED" == "true" ]]; then
    # Contract fulfilled — all expected Dogs present. Silent exit.
    exit 0
fi

# ── Contract breached — identify missing Dogs ──
EXPECTED_COUNT=$(echo "$HEALTH" | jq '.contract.expected_count // 0' 2>/dev/null || echo 0)
ACTIVE_DOGS=$(echo "$HEALTH" | jq '.dogs | length // 0' 2>/dev/null || echo 0)
MISSING_DOGS=$(echo "$HEALTH" | jq -r '.contract.missing_dogs[]? // empty' 2>/dev/null)

echo "$(date): Contract breach: $ACTIVE_DOGS/$EXPECTED_COUNT Dogs (missing: $MISSING_DOGS)"

# ── Restart local sovereign backends for missing Dogs ──
RESTARTED=0
for dog in $MISSING_DOGS; do
    # Only restart Dogs that run on THIS machine (sovereign, local systemd).
    # API-based Dogs (qwen-7b-hf) and remote Dogs (qwen35-9b-gpu) are not restartable here.
    case "$dog" in
        qwen25-7b-core)
            echo "  → Restarting llama-server (sovereign, local)"
            systemctl --user restart llama-server.service 2>/dev/null || \
                echo "  ✗ Could not restart llama-server"
            RESTARTED=1
            ;;
        deterministic-dog)
            echo "  → deterministic-dog is in-kernel; check kernel logs"
            ;;
        *)
            echo "  → $dog: not locally restartable (API/remote)"
            ;;
    esac
done

if [[ "$RESTARTED" -eq 1 ]]; then
    # Wait for backend to boot, then verify
    sleep 3
    HEALTH_AFTER=$(timeout 5 curl -s --max-time 5 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/health" 2>/dev/null || echo '{}')
    FULFILLED_AFTER=$(echo "$HEALTH_AFTER" | jq -r '.contract.fulfilled // false' 2>/dev/null)
    ACTIVE_AFTER=$(echo "$HEALTH_AFTER" | jq '.dogs | length // 0' 2>/dev/null || echo 0)

    if [[ "$FULFILLED_AFTER" == "true" ]]; then
        echo "$(date): Recovery successful: $ACTIVE_AFTER/$EXPECTED_COUNT Dogs online"
    else
        echo "$(date): Recovery incomplete: $ACTIVE_AFTER/$EXPECTED_COUNT"
        timeout 3 curl -s --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
            -H "Content-Type: application/json" \
            ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
            -d "{\"agent_id\":\"dog-health-monitor\",\"tool\":\"dog_restart\",\"target\":\"recovery_failed\",\"domain\":\"infra\",\"context\":\"expected=${EXPECTED_COUNT} actual=${ACTIVE_AFTER}\"}" \
            > /dev/null 2>&1 || true
    fi
fi
