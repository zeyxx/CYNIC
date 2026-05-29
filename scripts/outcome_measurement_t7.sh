#!/bin/bash
# outcome_measurement_t7.sh — K15 Closure: Compare baseline vs enriched verdicts
#
# Purpose: Measure token enrichment effectiveness by comparing verdicts before/after
# Timeline: Baseline collected 2026-05-25, enrichment deployed 2026-05-29, measure T+7 on 2026-06-02
# Input: 45 token mints + baseline verdicts (from k15-baseline observations)
# Output: divergence.csv + measurement_report.md
#
# Falsify: If divergence ≤ 0.05 on Q-score, enrichment is noise (not working)

set -euo pipefail

# ── Configuration ──
CYNIC_REST="${CYNIC_REST_ADDR:-http://127.0.0.1:3030}"
API_KEY="${CYNIC_API_KEY:-$(grep CYNIC_API_KEY ~/.cynic-env | cut -d= -f2)}"
OUTPUT_DIR="${OUTPUT_DIR:-.}"
TIMESTAMP="$(date +%Y-%m-%d-%H%M%S)"
DIVERGENCE_CSV="${OUTPUT_DIR}/divergence_${TIMESTAMP}.csv"
REPORT_MD="${OUTPUT_DIR}/measurement_report_${TIMESTAMP}.md"

echo "[outcome_measurement_t7] Starting K15 closure measurement..."
echo "[outcome_measurement_t7] Output: ${DIVERGENCE_CSV}"
echo "[outcome_measurement_t7] Report: ${REPORT_MD}"

# ── Helper: Query baseline verdicts from SurrealDB ──
# Expects observations tagged "k15-baseline" from 2026-05-25
# Each observation contains: token mint, baseline verdict (verdict_id, q_score, verdict_kind)
fetch_baseline_verdicts() {
    local query="SELECT verdict_id, mint, q_score, verdict_kind FROM observations WHERE tags CONTAINS 'k15-baseline' AND timestamp >= '2026-05-25T00:00:00Z' LIMIT 1000"

    # Use curl to POST to SurrealDB (or read from observations store via REST)
    # For now, use REST endpoint to fetch observations
    curl -s -X POST "${CYNIC_REST}/query" \
        -H "Authorization: Bearer ${API_KEY}" \
        -H "Content-Type: application/json" \
        -d "{\"query\":\"${query}\"}" 2>/dev/null || echo "[]"
}

# ── Helper: Build enriched stimulus from token mint ──
# Constructs token-analysis domain stimulus with enrichment hints
build_enriched_stimulus() {
    local mint="$1"
    # Stimulus includes mint address, triggers enrichment pathway in judge
    # token_postprocessor will apply class-aware adjustments
    echo "MINT:${mint} token-analysis enriched stimulus"
}

# ── Helper: Call /judge endpoint ──
# Returns: verdict_id, q_score, verdict_kind, dog_scores
judge_token() {
    local mint="$1"
    local stimulus=$(build_enriched_stimulus "$mint")

    local response=$(curl -s -X POST "${CYNIC_REST}/judge" \
        -H "Authorization: Bearer ${API_KEY}" \
        -H "Content-Type: application/json" \
        -d "{\"content\":\"${stimulus}\",\"domain\":\"token-analysis\"}" 2>/dev/null)

    # Extract: verdict_id, q_score.total, verdict_kind
    echo "$response" | jq -r '.verdict_id, .q_score.total, .verdict_kind, (.dog_scores | length)' 2>/dev/null || echo "null null null 0"
}

# ── Main measurement loop ──

# CSV header
echo "mint,baseline_verdict_id,baseline_q_score,baseline_verdict_kind,enriched_verdict_id,enriched_q_score,enriched_verdict_kind,q_score_delta,verdict_changed,dogs_voted" > "${DIVERGENCE_CSV}"

# Fetch baseline verdicts (mock for now, will be real data from 2026-05-25)
echo "[outcome_measurement_t7] Fetching baseline verdicts..."
baselines=$(fetch_baseline_verdicts)

# Count baselines
baseline_count=$(echo "$baselines" | jq '. | length' 2>/dev/null || echo 0)
echo "[outcome_measurement_t7] Found ${baseline_count} baseline verdicts (expected 45)"

# If baseline count < 45, warn but continue (may be test run)
if [ "$baseline_count" -lt 45 ]; then
    echo "[outcome_measurement_t7] WARNING: Expected 45 baseline verdicts, found ${baseline_count}"
fi

# Iterate baselines and measure enriched verdicts
total=0
divergence_sum=0.0
verdict_changes=0

echo "[outcome_measurement_t7] Measuring enriched verdicts..."

# For each baseline, measure enriched verdict
# (Mock loop — real version reads from baselines array)
for i in $(seq 1 ${baseline_count}); do
    # Extract baseline record
    baseline_verdict_id=$(echo "$baselines" | jq -r ".[$((i-1))].verdict_id" 2>/dev/null || echo "null")
    baseline_mint=$(echo "$baselines" | jq -r ".[$((i-1))].mint" 2>/dev/null || echo "null")
    baseline_q=$(echo "$baselines" | jq -r ".[$((i-1))].q_score" 2>/dev/null || echo "0.0")
    baseline_kind=$(echo "$baselines" | jq -r ".[$((i-1))].verdict_kind" 2>/dev/null || echo "null")

    if [ "$baseline_mint" = "null" ]; then
        continue
    fi

    echo -ne "\r[outcome_measurement_t7] Token $((i))/${baseline_count}                "

    # Measure enriched verdict
    read enriched_verdict_id enriched_q enriched_kind dogs_voted < <(judge_token "$baseline_mint")

    # Compute divergence
    q_delta=$(echo "$enriched_q - $baseline_q" | bc -l 2>/dev/null || echo "0")
    q_delta_abs=$(echo "scale=6; if ($q_delta < 0) -1 * $q_delta else $q_delta" | bc -l 2>/dev/null || echo "0")

    # Check verdict change
    verdict_changed=0
    if [ "$baseline_kind" != "$enriched_kind" ] && [ "$baseline_kind" != "null" ]; then
        verdict_changed=1
        ((verdict_changes++)) || true
    fi

    # Accumulate divergence
    divergence_sum=$(echo "$divergence_sum + $q_delta_abs" | bc -l 2>/dev/null || echo "$divergence_sum")

    # Write row
    echo "${baseline_mint},${baseline_verdict_id},${baseline_q},${baseline_kind},${enriched_verdict_id},${enriched_q},${enriched_kind},${q_delta},${verdict_changed},${dogs_voted}" >> "${DIVERGENCE_CSV}"

    ((total++)) || true
done

echo ""
echo "[outcome_measurement_t7] Measurement complete. ${total} tokens measured."

# ── Summary statistics ──

mean_divergence=$(echo "scale=6; $divergence_sum / $total" | bc -l 2>/dev/null || echo "0")
verdict_change_pct=$(echo "scale=2; $verdict_changes * 100 / $total" | bc -l 2>/dev/null || echo "0")

# ── Generate markdown report ──

cat > "${REPORT_MD}" << EOF
# K15 Closure: Outcome Measurement Report
**Date**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Measurement Period**: 2026-05-25 (baseline) → 2026-06-02 (T+7 enriched)

## Summary

| Metric | Value |
|--------|-------|
| Tokens Measured | ${total} |
| Mean Q-Score Divergence | ${mean_divergence} |
| Verdict Changes | ${verdict_changes}/${total} (${verdict_change_pct}%) |
| Baseline Count | ${baseline_count} |

## Falsification Test

**Hypothesis**: Token enrichment (post-processor + class-aware caps) improves verdict quality.

**Test**: If mean Q-score divergence ≤ 0.05, enrichment has no effect (is dead code).

**Result**:
- Mean divergence: **${mean_divergence}**
- **Status**: $(if (( $(echo "$mean_divergence > 0.05" | bc -l) )); then echo "PASS — enrichment shows effect"; else echo "FAIL — enrichment is noise"; fi)

## Data

Full divergence data in: \`${DIVERGENCE_CSV}\`

## Next Steps

1. If PASS: Proceed to Phase 2.1 (calibration loop)
2. If FAIL: Debug enrichment pipeline (token_postprocessor not applying caps correctly)

---

**Generated by**: outcome_measurement_t7.sh (2026-05-29)
EOF

echo "[outcome_measurement_t7] Report written: ${REPORT_MD}"
cat "${REPORT_MD}"

echo "[outcome_measurement_t7] Done."
exit 0
