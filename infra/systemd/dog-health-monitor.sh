#!/usr/bin/env bash
# CYNIC Dog Health Monitor — K15 Acting Consumer
# Polls kernel health. If Dogs < 4, restarts llama-server backends.
# Runs every 30s via systemd timer.
#
# K15: Sensor (kernel /health) must have acting consumer.
# Consumer: Auto-restart backends on degradation.
# Outcome: CYNIC self-heals common failures without human intervention.

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
HEALTH=$(curl -s --max-time 5 \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    "http://${KERNEL_ADDR}/health" 2>/dev/null || echo '{}')

# Extract Dog count
ACTIVE_DOGS=$(echo "$HEALTH" | jq '.dogs | length // 0' 2>/dev/null || echo 0)
EXPECTED_DOGS=4

if [[ "$ACTIVE_DOGS" -lt "$EXPECTED_DOGS" ]]; then
    MISSING=$((EXPECTED_DOGS - ACTIVE_DOGS))

    echo "$(date): Dog degradation detected: $ACTIVE_DOGS/$EXPECTED_DOGS (missing $MISSING)"

    # ── Identify which Dogs are missing ──
    ACTIVE_DOG_IDS=$(echo "$HEALTH" | jq -r '.dogs[].id // empty' 2>/dev/null | sort)
    EXPECTED="deterministic-dog
gemma-4b-core
qwen-7b-hf
qwen35-9b-gpu"

    for dog in $EXPECTED; do
        if ! echo "$ACTIVE_DOG_IDS" | grep -q "^${dog}$"; then
            echo "  Missing: $dog"

            # Restart the appropriate backend
            case "$dog" in
                gemma-4b-core)
                    echo "  → Restarting gemma-4b (llama-server on :8080)"
                    systemctl --user restart llama-server@gemma-4b.service 2>/dev/null || \
                        ssh -i ~/.ssh/id_ed25519 -o ConnectTimeout=3 user@${CYNIC_REST_ADDR%:*} \
                            "systemctl --user restart llama-server@gemma-4b.service" || \
                        echo "  ✗ Could not restart gemma-4b"
                    ;;
                qwen-7b-hf)
                    echo "  → qwen-7b-hf is API-based (HF); check network/auth"
                    ;;
                qwen35-9b-gpu)
                    echo "  → Restarting qwen35-9b on GPU node"
                    ssh -i ~/.ssh/id_ed25519 -o ConnectTimeout=3 titou@100.119.192.107 \
                        "taskkill /im llama-server.exe /f 2>/dev/null || true; sleep 2; taskkill /im cmd.exe /f 2>/dev/null || true" || \
                        echo "  ✗ Could not reach GPU node"
                    ;;
                deterministic-dog)
                    echo "  → deterministic-dog is in-kernel; check kernel logs"
                    ;;
            esac
        fi
    done

    # Wait for backends to boot
    sleep 3

    # Verify recovery
    HEALTH_AFTER=$(curl -s --max-time 5 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/health" 2>/dev/null || echo '{}')
    ACTIVE_DOGS_AFTER=$(echo "$HEALTH_AFTER" | jq '.dogs | length // 0' 2>/dev/null || echo 0)

    if [[ "$ACTIVE_DOGS_AFTER" -ge "$EXPECTED_DOGS" ]]; then
        echo "$(date): Recovery successful: $ACTIVE_DOGS_AFTER/$EXPECTED_DOGS Dogs online"
    else
        echo "$(date): Recovery incomplete: $ACTIVE_DOGS_AFTER/$EXPECTED_DOGS (still missing $((EXPECTED_DOGS - ACTIVE_DOGS_AFTER)))"
        echo "  → Escalate to human: Dogs not recovering automatically"

        # Could POST to /observe here for visibility
        curl -s --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
            -H "Content-Type: application/json" \
            ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
            -d "{\"agent_id\":\"dog-health-monitor\",\"tool\":\"dog_restart\",\"target\":\"recovery_failed\",\"domain\":\"infra\",\"context\":\"expected=${EXPECTED_DOGS} actual=${ACTIVE_DOGS_AFTER}\"}" \
            > /dev/null 2>&1 || true
    fi
else
    # Healthy state — silent
    :
fi
