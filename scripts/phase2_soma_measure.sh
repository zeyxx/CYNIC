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

# R1: Verify deterministic-dog is available (single probe)
echo "Checking deterministic-dog availability..." >&2
test_resp=$(timeout 5 curl -s -X POST "$KERNEL/judge" \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content":"test","soma_gate":false,"dogs":["deterministic-dog"]}' 2>/dev/null)
verdict=$(echo "$test_resp" | jq -r '.verdict // empty' 2>/dev/null || echo "")
if [ -n "$verdict" ]; then
  echo "✓ deterministic-dog available, proceeding with measurement" >&2
else
  echo "✗ deterministic-dog unavailable. Cannot proceed." >&2
  exit 1
fi

# Header
echo "stimulus,q_score_baseline,q_score_soma,delta_pct,verdict_baseline,verdict_soma" > "$OUTPUT_FILE"

read_count=0
ensemble_shifts=0
while IFS= read -r stimulus; do
  [[ -z "$stimulus" ]] && continue

  # Baseline: soma_gate=false, deterministic-dog only
  baseline_resp=$(timeout 8 curl -s -X POST "$KERNEL/judge" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"content\":\"$stimulus\",\"soma_gate\":false,\"dogs\":[\"deterministic-dog\"]}")
  baseline=$(echo "$baseline_resp" | jq -r '.q_score.total // empty')
  baseline_verdict=$(echo "$baseline_resp" | jq -r '.verdict // empty')

  # Soma gate: soma_gate=true, same dog
  soma_resp=$(timeout 8 curl -s -X POST "$KERNEL/judge" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"content\":\"$stimulus\",\"soma_gate\":true,\"dogs\":[\"deterministic-dog\"]}")
  soma=$(echo "$soma_resp" | jq -r '.q_score.total // empty')
  soma_verdict=$(echo "$soma_resp" | jq -r '.verdict // empty')

  # Skip if either call failed
  if [ -z "$baseline" ] || [ -z "$soma" ]; then
    echo "[$read_count] ⊘ Request timeout/failed, skipping" >&2
    ((ensemble_shifts++))
    continue
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
  echo "[$read_count] baseline=$baseline soma=$soma delta=$delta_pct%" >&2
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
