#!/bin/bash
# Phase 2 Soma Gate Measurement
# Usage: <token-list.json jq '.[] | .content'> | phase2_soma_measure.sh
# Output: CSV with baseline, soma_gate, delta_pct per token
# R23-exempt: Script naturally needs to reference CYNIC_API_KEY env var (not embedding secret)

set -euo pipefail

KERNEL="${CYNIC_REST_ADDR:-http://localhost:3030}"
SOMA_L2="${SOMA_L2_ADDR:-http://127.0.0.1:5555}"
# Auth token: must be set in environment before running (see ~/.cynic-env)
if [ -z "${CYNIC_API_KEY:-}" ]; then
  echo "Error: CYNIC_API_KEY not set" >&2
  exit 1
fi
AUTH_TOKEN="$CYNIC_API_KEY"
OUTPUT_FILE="${1:-.soma_phase2_results.csv}"

# R1: Verify required Dogs are available before starting measurement
REQUIRED_DOGS="deterministic-dog,qwen35-9b-gpu"
echo "Checking Dog availability via Soma L2..." >&2
check_resp=$(curl -s -f "$SOMA_L2/soma/check-dog-availability?dogs=$REQUIRED_DOGS" 2>/dev/null || echo '{"available":false,"reason":"L2 unreachable"}')
available=$(echo "$check_resp" | jq -r '.available // false')
if [ "$available" != "true" ]; then
  reason=$(echo "$check_resp" | jq -r '.reason // "unknown"')
  echo "✗ Required Dogs unavailable: $reason" >&2
  echo "Waiting 30s for cluster stabilization..." >&2
  sleep 30
  # Retry once
  check_resp=$(curl -s -f "$SOMA_L2/soma/check-dog-availability?dogs=$REQUIRED_DOGS" 2>/dev/null || echo '{"available":false}')
  available=$(echo "$check_resp" | jq -r '.available // false')
  if [ "$available" != "true" ]; then
    echo "✗ Dogs still unavailable. Cannot proceed with measurement." >&2
    exit 1
  fi
fi
echo "✓ Required Dogs available, proceeding with measurement" >&2

# Header
echo "stimulus,q_score_baseline,q_score_soma,delta_pct,verdict_baseline,verdict_soma" > "$OUTPUT_FILE"

read_count=0
ensemble_shifts=0
while IFS= read -r stimulus; do
  [[ -z "$stimulus" ]] && continue

  # R2: Check Dog availability before each measurement (detect degradation)
  check_resp=$(curl -s -f "$SOMA_L2/soma/check-dog-availability?dogs=$REQUIRED_DOGS" 2>/dev/null || echo '{"available":false}')
  available=$(echo "$check_resp" | jq -r '.available // false')
  if [ "$available" != "true" ]; then
    echo "[$read_count] ✗ Dogs became unavailable during measurement, aborting" >&2
    break
  fi

  # Baseline: soma_gate=false, locked ensemble to stable Dogs only
  # (deterministic-dog + qwen35-9b-gpu to prevent gemini-cli/qwen-7b-hf flakiness)
  baseline_resp=$(curl -s -X POST "$KERNEL/judge" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"content\":\"$stimulus\",\"soma_gate\":false,\"dogs\":[\"deterministic-dog\",\"qwen35-9b-gpu\"]}")
  baseline=$(echo "$baseline_resp" | jq -r '.q_score.total // empty')
  baseline_verdict=$(echo "$baseline_resp" | jq -r '.verdict // empty')
  baseline_dogs=$(echo "$baseline_resp" | jq -r '.dogs_used | length // 0')

  # Soma gate: soma_gate=true, same locked ensemble
  soma_resp=$(curl -s -X POST "$KERNEL/judge" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"content\":\"$stimulus\",\"soma_gate\":true,\"dogs\":[\"deterministic-dog\",\"qwen35-9b-gpu\"]}")
  soma=$(echo "$soma_resp" | jq -r '.q_score.total // empty')
  soma_verdict=$(echo "$soma_resp" | jq -r '.verdict // empty')
  soma_dogs=$(echo "$soma_resp" | jq -r '.dogs_used | length // 0')

  # R3: Detect ensemble shifts (different number of Dogs used)
  if [ "$baseline_dogs" != "$soma_dogs" ]; then
    echo "[$read_count] ✗ Ensemble shift detected: baseline=$baseline_dogs dogs, soma=$soma_dogs dogs" >&2
    ((ensemble_shifts++))
    continue  # Skip this measurement
  fi

  # Compute delta
  if (( $(echo "$baseline > 0" | bc -l) )); then
    delta_pct=$(echo "scale=2; ($soma - $baseline) / $baseline * 100" | bc -l)
  else
    delta_pct=0
  fi

  # Truncate stimulus for CSV
  stimulus_short=$(echo "$stimulus" | cut -c1-80)

  echo "$stimulus_short,$baseline,$soma,$delta_pct,$baseline_verdict,$soma_verdict" >> "$OUTPUT_FILE"

  ((read_count++))
  echo "[$read_count] baseline=$baseline soma=$soma delta=$delta_pct% dogs=$baseline_dogs" >&2
done

# Summary stats
echo "" >&2
echo "=== Phase 2 Results ===" >&2
echo "Ensemble shifts detected: $ensemble_shifts (excluded from analysis)" >&2
awk -F',' 'NR>1 {sum+=$4; count++; if($4>5) improved++} END {
  if(count>0) {
    mean=sum/count
    pct_improved=(improved/count)*100
    print "Stable measurements: " count
    print "Mean delta: " mean "%"
    print "Verdicts improved >5%: " pct_improved "%"
    if(mean>5) print "✓ HYPOTHESIS VALIDATED (mean delta > 5%)" > "/dev/stderr"
    else print "✗ No significant improvement (mean delta <= 5%)" > "/dev/stderr"
  }
}' "$OUTPUT_FILE"

echo "Results saved to: $OUTPUT_FILE" >&2
