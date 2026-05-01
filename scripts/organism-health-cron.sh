#!/bin/bash
# Organism Health Cron — Run every 30min, trigger Haiku only on real divergence
# Install: `*/30 * * * * /home/user/Bureau/CYNIC/scripts/organism-health-cron.sh >> /var/log/organism-health.log 2>&1`

set -e

CYNIC_ROOT="$(git rev-parse --show-toplevel)"
STATE_FILE="/tmp/organism_state.json"
STATE_PREV="/tmp/organism_state.previous.json"
THRESHOLD_SIZE=200  # Only trigger Haiku if diff > 200 bytes (noise filtering)

cd "$CYNIC_ROOT"

# Run probe
bash scripts/organism-probe.sh > /dev/null 2>&1

# Compare with previous state
if [ -f "$STATE_PREV" ]; then
  DIFF_SIZE=$(diff <(jq -S . "$STATE_PREV" 2>/dev/null || echo "{}") <(jq -S . "$STATE_FILE" 2>/dev/null || echo "{}") | wc -c)

  if [ "$DIFF_SIZE" -gt "$THRESHOLD_SIZE" ]; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Divergence detected ($DIFF_SIZE bytes) → triggering Haiku"

    # Trigger Haiku agent (non-blocking)
    # Note: This is a placeholder. Actual integration depends on Claude Code agent dispatch mechanism.
    # For now, write a marker file that Claude Code can pick up.
    cat > /tmp/organism_health_trigger.txt << EOF
timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
previous_state=$(cat "$STATE_PREV")
current_state=$(cat "$STATE_FILE")
action=synthesize_and_update_memory
EOF
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Haiku trigger written to /tmp/organism_health_trigger.txt"
  else
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] No significant divergence ($DIFF_SIZE bytes < $THRESHOLD_SIZE)"
  fi
else
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] First run, baseline established"
fi

# Save state for next iteration
cp "$STATE_FILE" "$STATE_PREV"
