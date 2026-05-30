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
CYNIC_REST="http://${CYNIC_REST_ADDR:-127.0.0.1:3030}"
API_KEY="${CYNIC_API_KEY:-$(grep CYNIC_API_KEY ~/.cynic-env | cut -d= -f2)}"
OUTPUT_DIR="${OUTPUT_DIR:-.}"
TIMESTAMP="$(date +%Y-%m-%d-%H%M%S)"
DIVERGENCE_CSV="${OUTPUT_DIR}/divergence_${TIMESTAMP}.csv"
REPORT_MD="${OUTPUT_DIR}/measurement_report_${TIMESTAMP}.md"

echo "[outcome_measurement_t7] Starting K15 closure measurement..."
echo "[outcome_measurement_t7] Output: ${DIVERGENCE_CSV}"
echo "[outcome_measurement_t7] Report: ${REPORT_MD}"

# ── Helper: Read baseline verdicts from local JSONL files ──
# K15 baseline collected 2026-05-25 to 2026-05-26 and stored in git commit 133aadd
# Each line is a JSON object with: mint, q_score_total, verdict_kind, baseline_captured_at
fetch_baseline_verdicts() {
    local verdict_dir="cynic-python/heuristics/data/verdicts"

    if [ ! -d "$verdict_dir" ]; then
        echo "✗ Verdict baselines not found at $verdict_dir" >&2
        echo "  (Restore from git: git show 133aadd:cynic-python/heuristics/data/verdicts/ > ...)" >&2
        return 1
    fi

    # Extract: mint, q_score_total, verdict_kind, captured_at, holder_count,
    #          concentration (0-1), mint_authority_active, freeze_authority_active
    # Some mints appear multiple times (re-evaluated); take latest by captured_at (col 4)
    jq -r '[
        .mint,
        .q_score_total,
        .verdict_kind,
        .baseline_captured_at,
        (.baseline_holder_count // 0 | tostring),
        (.baseline_concentration // 0 | tostring),
        (.baseline_mint_authority_active // false | tostring),
        (.baseline_freeze_authority_active // false | tostring)
    ] | @tsv' \
        "$verdict_dir"/verdicts_2026-05-*.jsonl 2>/dev/null | \
        sort -t$'\t' -k4 -r | sort -t$'\t' -u -k1,1
}

# ── Helper: Build enriched stimulus from baseline JSONL fields ──
# Option A: Reconstruct from baseline data — isolates the postprocessor variable.
# Same input data as 2026-05-25, only the pipeline (token_postprocessor) has changed.
# Missing fields (age_hours, lp_status, name, symbol) use safe defaults.
#
# content → Dogs read [DOMAIN: token-analysis]\n[METRICS]\n...
# context → token_postprocessor reads (age_days, mint_authority, freeze_authority, etc.)
build_enriched_stimulus() {
    local mint="$1"
    local holder_count="$2"
    local concentration_raw="$3"   # 0-1 decimal from baseline JSONL
    local mint_auth="$4"           # "true" / "false"
    local freeze_auth="$5"         # "true" / "false"

    # Convert concentration 0-1 → percentage for stimulus format
    local concentration_pct
    concentration_pct=$(echo "scale=2; ${concentration_raw} * 100" | bc -l 2>/dev/null || echo "0.00")

    local mint_auth_str="REVOKED (supply is fixed)"
    local freeze_auth_str="REVOKED (wallets are free)"
    [ "$mint_auth" = "true" ] && mint_auth_str="ACTIVE (can mint more tokens)"
    [ "$freeze_auth" = "true" ] && freeze_auth_str="ACTIVE (can freeze wallets)"

    cat <<EOF
[DOMAIN: token-analysis]

[METRICS]
mint: ${mint}
holders: ${holder_count}
top_10_wallets_pct: ${concentration_pct}%
mint_authority: ${mint_auth_str}
freeze_authority: ${freeze_auth_str}
lp_secured: UNKNOWN — baseline snapshot, LP data unavailable
age_hours: UNKNOWN — baseline snapshot (age not recorded)

[AXIOM EVIDENCE]
FIDELITY: Holder count ${holder_count}, concentration data from 2026-05-25 baseline snapshot.
BURN: Top-10 wallets hold ${concentration_pct}% of supply.
SOVEREIGNTY: Mint authority ${mint_auth_str}. Freeze authority ${freeze_auth_str}.

[QUESTION]
Is this Solana token trustworthy based on available on-chain metrics?
EOF
}

# ── Helper: Call /judge endpoint with reconstructed stimulus ──
# Returns: verdict_id, q_score, verdict_kind, dog_scores_count
judge_token() {
    local mint="$1"
    local holder_count="$2"
    local concentration_raw="$3"
    local mint_auth="$4"
    local freeze_auth="$5"

    local mint_auth_label="REVOKED"
    local freeze_auth_label="REVOKED"
    [ "$mint_auth" = "true" ] && mint_auth_label="ACTIVE"
    [ "$freeze_auth" = "true" ] && freeze_auth_label="ACTIVE"

    local concentration_pct
    concentration_pct=$(echo "scale=2; ${concentration_raw} * 100" | bc -l 2>/dev/null || echo "0.00")

    local content
    content=$(build_enriched_stimulus "$mint" "$holder_count" "$concentration_raw" "$mint_auth" "$freeze_auth")

    # context field: token_postprocessor reads this for class-aware caps
    local context="mint: ${mint}
holder_count: ${holder_count}
top_10_wallets_pct: ${concentration_pct}
mint_authority: ${mint_auth_label}
freeze_authority: ${freeze_auth_label}"

    local response
    response=$(curl -s --max-time 120 -X POST "${CYNIC_REST}/judge" \
        -H "Authorization: Bearer ${API_KEY}" \
        -H "Content-Type: application/json" \
        -d "$(jq -n \
            --arg content "$content" \
            --arg context "$context" \
            --arg domain "token-analysis" \
            '{content: $content, context: $context, domain: $domain}')" 2>/dev/null)

    # Single TAB-separated line so the caller's `read` captures all 4 fields.
    # (jq with comma emits 4 newlines; `read` only consumes the first line → 3 fields lost.)
    # verdict KIND lives in top-level `.verdict` ("Growl"/"Wag"/"Epoche"); `.verdict_kind` does not exist.
    echo "$response" | jq -r '[.verdict_id, (.q_score.total | tostring), .verdict, (.dog_scores | length | tostring)] | @tsv' 2>/dev/null \
        || printf 'null\tnull\tnull\t0\n'
}

# ── Main measurement loop ──

# CSV header
echo "mint,baseline_verdict_id,baseline_q_score,baseline_verdict_kind,enriched_verdict_id,enriched_q_score,enriched_verdict_kind,q_score_delta,verdict_changed,dogs_voted" > "${DIVERGENCE_CSV}"

# Fetch baseline verdicts from local JSONL files
echo "[outcome_measurement_t7] Fetching baseline verdicts..."
baselines=$(fetch_baseline_verdicts)

# Count baselines (now TSV lines, not JSON array)
baseline_count=$(echo "$baselines" | wc -l)
echo "[outcome_measurement_t7] Found ${baseline_count} baseline verdicts (expected 45)"

# If baseline count < 45, warn but continue (may be test run)
if [ "$baseline_count" -lt 45 ]; then
    echo "[outcome_measurement_t7] WARNING: Expected 45 baseline verdicts, found ${baseline_count}"
fi

# Iterate baselines and measure enriched verdicts
total=0
divergence_sum=0.0
verdict_changes=0
error_count=0

echo "[outcome_measurement_t7] Measuring enriched verdicts..."

# Parse TSV lines: mint, q_score_total, verdict_kind, captured_at,
#                  holder_count, concentration, mint_auth_active, freeze_auth_active
while IFS=$'\t' read -r baseline_mint baseline_q baseline_kind baseline_ts \
                         baseline_holders baseline_concentration baseline_mint_auth baseline_freeze_auth; do
    if [ -z "$baseline_mint" ] || [ "$baseline_mint" = "null" ]; then
        continue
    fi

    echo -ne "\r[outcome_measurement_t7] Token $((total+1))/${baseline_count}                "

    # Measure enriched verdict using reconstructed stimulus from baseline fields
    IFS=$'\t' read -r enriched_verdict_id enriched_q enriched_kind dogs_voted < <(judge_token \
        "$baseline_mint" \
        "${baseline_holders:-0}" \
        "${baseline_concentration:-0}" \
        "${baseline_mint_auth:-false}" \
        "${baseline_freeze_auth:-false}")

    # GUARD: enriched side must be a real number. A missing/null enriched Q-score is a
    # measurement ERROR (judge failed or parsing broke), NOT a divergence. Without this guard,
    # an empty enriched_q makes `bc "$enriched_q - $baseline_q"` evaluate " - baseline" (unary
    # minus) → delta == -baseline → fake maximal divergence → false PASS. (Root cause, 2026-05-29.)
    if ! [[ "$enriched_q" =~ ^-?[0-9]+([.][0-9]+)?$ ]]; then
        echo "${baseline_mint},N/A,${baseline_q},${baseline_kind},${enriched_verdict_id:-null},ERROR,ERROR,ERROR,ERROR,${dogs_voted:-0}" >> "${DIVERGENCE_CSV}"
        ((error_count++)) || true
        ((total++)) || true
        continue
    fi

    # Compute divergence (absolute change in Q-score)
    q_delta=$(echo "$enriched_q - $baseline_q" | bc -l 2>/dev/null || echo "0")
    q_delta_abs=$(echo "scale=6; if ($q_delta < 0) -1 * $q_delta else $q_delta" | bc -l 2>/dev/null || echo "0")

    # Check verdict change (top-level `.verdict` vs baseline kind)
    verdict_changed=0
    if [ "$baseline_kind" != "$enriched_kind" ] && [ "$baseline_kind" != "null" ]; then
        verdict_changed=1
        ((verdict_changes++)) || true
    fi

    # Accumulate divergence sum
    divergence_sum=$(echo "$divergence_sum + $q_delta_abs" | bc -l 2>/dev/null || echo "$divergence_sum")

    # Write CSV row (baseline_verdict_id = N/A since we don't have it from JSONL)
    echo "${baseline_mint},N/A,${baseline_q},${baseline_kind},${enriched_verdict_id},${enriched_q},${enriched_kind},${q_delta},${verdict_changed},${dogs_voted}" >> "${DIVERGENCE_CSV}"

    ((total++)) || true
done <<< "$baselines"

echo ""
echo "[outcome_measurement_t7] Measurement complete. ${total} tokens measured."

# ── Summary statistics ──

# Divergence is meaningful only over tokens that produced a real enriched verdict.
measured=$(( total - error_count ))
if [ "$measured" -le 0 ]; then
    mean_divergence="0"
    verdict_change_pct="0"
else
    mean_divergence=$(echo "scale=6; $divergence_sum / $measured" | bc -l 2>/dev/null || echo "0")
    verdict_change_pct=$(echo "scale=2; $verdict_changes * 100 / $measured" | bc -l 2>/dev/null || echo "0")
fi

# ── Generate markdown report ──

cat > "${REPORT_MD}" << EOF
# K15 Closure: Outcome Measurement Report
**Date**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Measurement Period**: 2026-05-25 (baseline) → 2026-06-02 (T+7 enriched)

## Summary

| Metric | Value |
|--------|-------|
| Tokens In Set | ${total} |
| Successfully Measured | ${measured} |
| Measurement Errors | ${error_count} |
| Mean Q-Score Divergence | ${mean_divergence} |
| Verdict Changes | ${verdict_changes}/${measured} (${verdict_change_pct}%) |
| Baseline Count | ${baseline_count} |

## Falsification Test

**Hypothesis**: Token enrichment (post-processor + class-aware caps) changes verdicts measurably.
(NOTE: this detects *change*, not *improvement* — a large divergence with no ground truth
cannot tell better from worse. Improvement requires labelled outcomes, not just divergence.)

**Test**: If mean Q-score divergence ≤ 0.05, enrichment has no effect (is dead code).
**Validity guard**: if >50% of tokens error out, the measurement itself is invalid → ERROR, not PASS.

**Result**:
- Successfully measured: **${measured}/${total}** (${error_count} errors)
- Mean divergence: **${mean_divergence}**
- **Status**: $(if [ "$measured" -le 0 ] || [ "$error_count" -gt $(( total / 2 )) ]; then echo "ERROR — measurement invalid (${error_count}/${total} tokens failed); fix before interpreting"; elif (( $(echo "$mean_divergence > 0.05" | bc -l) )); then echo "PASS — enrichment shows effect"; else echo "FAIL — enrichment is noise"; fi)

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
