#!/bin/bash
# Monitor Hermes CCM validation run
# Collects: observations, crystals, GPU load, Dog timeouts
# Duration: 4-6h (let 1-2 cron cycles complete)
# Output: JSON log for analysis

set -euo pipefail

# Load from environment — expects $CYNIC_REST_ADDR and $CYNIC_API_KEY
# to be exported before running this script (from ~/.cynic-env via source)
CYNIC_REST_ADDR="${CYNIC_REST_ADDR:-http://127.0.0.1:3030}"
if [ -z "${CYNIC_API_KEY:-}" ]; then
    echo "Error: CYNIC_API_KEY not set. Run: source ~/.cynic-env"
    exit 1
fi
MONITOR_DIR="${1:-.}"
LOGFILE="${MONITOR_DIR}/hermes-validation-$(date +%s).jsonl"

# Create output directory
mkdir -p "$MONITOR_DIR"
touch "$LOGFILE"

echo "Monitoring Hermes CCM validation to: $LOGFILE"
echo "Duration: ~4-6h (until 2 cron cycles complete)"
echo "Metrics: observations, crystals, q_score, GPU load"
echo ""

collect_metrics() {
    local timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    # Observation count
    local obs_count=$(curl -s -H "Authorization: Bearer $CYNIC_API_KEY" \
        "$CYNIC_REST_ADDR/observations?limit=1000" 2>/dev/null | jq 'length' 2>/dev/null || echo "null")

    # Hermes observations specifically
    local hermes_obs=$(curl -s -H "Authorization: Bearer $CYNIC_API_KEY" \
        "$CYNIC_REST_ADDR/observations?limit=1000" 2>/dev/null | jq '[.[] | select(.agent_id | contains("hermes"))] | length' 2>/dev/null || echo "null")

    # Crystal count (forming vs crystallized)
    local crystals=$(curl -s -H "Authorization: Bearer $CYNIC_API_KEY" \
        "$CYNIC_REST_ADDR/crystals?limit=100" 2>/dev/null | jq '{forming: [.[] | select(.status=="forming")] | length, crystallized: [.[] | select(.status=="crystallized")] | length}' 2>/dev/null || echo "null")

    # Health check (includes q_score, dog_scores, timeouts)
    local health=$(curl -s -H "Authorization: Bearer $CYNIC_API_KEY" \
        "$CYNIC_REST_ADDR/health" 2>/dev/null | jq '{kernel_status: .status, dogs_count: .dogs | length, dog_scores: .dogs | map(.score) | add / length}' 2>/dev/null || echo "null")

    # GPU load (if available via llama-server health on cynic-gpu)
    local gpu_load=$(curl -s "http://${LLAMA_SERVER_HOST:-<TAILSCALE_GPU>}:8080/health" 2>/dev/null | jq '.slots | map(.n_tokens) | add' 2>/dev/null || echo "null")

    # Write to log
    jq -n \
        --arg timestamp "$timestamp" \
        --argjson obs_count "$obs_count" \
        --argjson hermes_obs "$hermes_obs" \
        --argjson crystals "$crystals" \
        --argjson health "$health" \
        --argjson gpu_load "$gpu_load" \
        '{timestamp, obs_count, hermes_obs, crystals, health, gpu_load}' >> "$LOGFILE"
}

# Collect every 30 seconds for the duration
COLLECT_COUNT=0
MAX_COLLECTIONS=480  # 4 hours at 30s interval

echo "Starting collection loop..."
while [ $COLLECT_COUNT -lt $MAX_COLLECTIONS ]; do
    collect_metrics
    echo -ne "\r[$(date +%H:%M:%S)] Collections: $COLLECT_COUNT/$MAX_COLLECTIONS | Hermes obs: $(tail -1 "$LOGFILE" | jq -r '.hermes_obs') | Crystals: $(tail -1 "$LOGFILE" | jq -r '.crystals')"
    COLLECT_COUNT=$((COLLECT_COUNT + 1))
    sleep 30
done

echo ""
echo ""
echo "Collection complete. Results in: $LOGFILE"
echo ""
echo "Analysis:"
jq -s '[.[] | select(.hermes_obs > 0)] | {
  start: .[0].timestamp,
  end: .[-1].timestamp,
  total_observations: [.[] | .obs_count] | max,
  hermes_observations: [.[] | .hermes_obs] | max,
  crystals_forming: [.[] | .crystals.forming] | max,
  crystals_crystallized: [.[] | .crystals.crystallized] | max,
  avg_dog_score: ([.[] | .health.dog_scores] | add / length)
}' "$LOGFILE"
