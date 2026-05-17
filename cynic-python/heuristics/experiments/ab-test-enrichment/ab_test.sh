#!/bin/bash
# A/B test: divergence enrichment impact on token judgment accuracy
# Baseline vs enriched (domain=token-analysis) accuracy comparison

set -e

# Configuration
KERNEL_URL="${CYNIC_REST_ADDR:-http://localhost:3030}"
API_KEY="${CYNIC_API_KEY}"
DATA_FILE="$(dirname "$0")/calibration_results_real.json"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "================================================================================"
echo "A/B TEST: Divergence Enrichment Impact"
echo "================================================================================"
echo "Baseline:  /judge without domain (conviction heuristics only)"
echo "Enriched:  /judge with domain='token-analysis' (+ divergence signal)"
echo "Ground truth: CultScreener conviction labels"
echo "Dog: deterministic-dog only (0ms latency, no contention)"
echo "================================================================================"

# Load tokens from calibration data
if [ ! -f "$DATA_FILE" ]; then
    echo "ERROR: $DATA_FILE not found"
    exit 1
fi

# Extract tokens and run A/B test
declare -a mints conviction_tiers expected_verdicts

# Parse JSON using jq
mapfile -t mints < <(jq -r '.results[].mint' "$DATA_FILE")
mapfile -t symbols < <(jq -r '.results[].symbol' "$DATA_FILE" | tr -d '[:space:]')
mapfile -t convictions < <(jq -r '.results[].conviction' "$DATA_FILE")
mapfile -t tiers < <(jq -r '.results[].conviction_tier' "$DATA_FILE")
mapfile -t expected < <(jq -r '.results[].expected_verdict' "$DATA_FILE")

n=${#mints[@]}
echo ""
echo "Testing $n tokens..."
echo ""

baseline_correct=0
enriched_correct=0
improvements=0
regressions=0
tested=0

# Results file
results_file="$(dirname "$0")/ab_test_results_$(date +%Y%m%d_%H%M%S).jsonl"

for i in "${!mints[@]}"; do
    mint="${mints[$i]}"
    symbol="${symbols[$i]}"
    conviction="${convictions[$i]}"
    tier="${tiers[$i]}"
    expected_verdict="${expected[$i]}"

    printf "[%2d/%d] %-15s (conv=%.3f)... " $((i+1)) $n "$symbol" "$conviction"

    # Baseline: no domain hint
    baseline_resp=$(curl -s -X POST "$KERNEL_URL/judge" \
        -H "Authorization: Bearer $API_KEY" \
        -H "Content-Type: application/json" \
        -d "{\"content\":\"$mint\",\"dogs\":[\"deterministic-dog\"]}")

    baseline_q_score=$(echo "$baseline_resp" | jq -r '.q_score.total // empty' 2>/dev/null)
    if [ -z "$baseline_q_score" ]; then
        echo "SKIP (baseline failed)"
        continue
    fi

    # Convert Q-score to verdict
    baseline_verdict=$(jq -n --arg qs "$baseline_q_score" \
        'if ($qs | tonumber) > 0.528 then "Howl"
         elif ($qs | tonumber) > 0.382 then "Wag"
         elif ($qs | tonumber) > 0.236 then "Growl"
         else "Bark" end')

    printf "B:%s " "$baseline_verdict"

    sleep 0.2

    # Enriched: with domain=token-analysis
    enriched_resp=$(curl -s -X POST "$KERNEL_URL/judge" \
        -H "Authorization: Bearer $API_KEY" \
        -H "Content-Type: application/json" \
        -d "{\"content\":\"$mint\",\"domain\":\"token-analysis\",\"dogs\":[\"deterministic-dog\"]}")

    enriched_q_score=$(echo "$enriched_resp" | jq -r '.q_score.total // empty' 2>/dev/null)
    if [ -z "$enriched_q_score" ]; then
        echo "SKIP (enriched failed)"
        continue
    fi

    # Convert Q-score to verdict
    enriched_verdict=$(jq -n --arg qs "$enriched_q_score" \
        'if ($qs | tonumber) > 0.528 then "Howl"
         elif ($qs | tonumber) > 0.382 then "Wag"
         elif ($qs | tonumber) > 0.236 then "Growl"
         else "Bark" end')

    printf "E:%s " "$enriched_verdict"

    # Score
    baseline_match=0
    enriched_match=0
    improvement=0

    if [ "$baseline_verdict" == "$expected_verdict" ]; then
        baseline_match=1
        ((baseline_correct++))
    fi

    if [ "$enriched_verdict" == "$expected_verdict" ]; then
        enriched_match=1
        ((enriched_correct++))
    fi

    if [ $enriched_match -eq 1 ] && [ $baseline_match -eq 0 ]; then
        improvement=1
        ((improvements++))
        echo -e "${GREEN}✓ IMPROVED${NC}"
    elif [ $enriched_match -eq 0 ] && [ $baseline_match -eq 1 ]; then
        ((regressions++))
        echo -e "${RED}✗ REGRESSED${NC}"
    else
        echo ""
    fi

    ((tested++))

    # Record result
    jq -n \
        --arg mint "$mint" \
        --arg symbol "$symbol" \
        --arg tier "$tier" \
        --arg conviction "$conviction" \
        --arg expected "$expected_verdict" \
        --arg baseline_v "$baseline_verdict" \
        --arg baseline_q "$baseline_q_score" \
        --arg enriched_v "$enriched_verdict" \
        --arg enriched_q "$enriched_q_score" \
        --argjson baseline_m "$baseline_match" \
        --argjson enriched_m "$enriched_match" \
        --argjson improved "$improvement" \
        '{mint: $mint, symbol: $symbol, tier: $tier, conviction: $conviction,
          expected_verdict: $expected,
          baseline_verdict: $baseline_v, baseline_q_score: $baseline_q, baseline_match: $baseline_m,
          enriched_verdict: $enriched_v, enriched_q_score: $enriched_q, enriched_match: $enriched_m,
          improvement: $improved}' >> "$results_file"

    sleep 0.2
done

echo ""
echo "================================================================================"
echo "RESULTS"
echo "================================================================================"

if [ $tested -eq 0 ]; then
    echo "ERROR: No successful test runs"
    exit 1
fi

baseline_acc=$(echo "scale=4; $baseline_correct / $tested" | bc)
enriched_acc=$(echo "scale=4; $enriched_correct / $tested" | bc)
improvement_pct=$(echo "scale=1; ($enriched_correct - $baseline_correct) / $tested * 100" | bc)

echo "N=$tested"
echo ""
echo "Baseline accuracy:  $baseline_correct/$tested = $(echo "scale=1%; $baseline_acc * 100" | bc)%"
echo "Enriched accuracy:  $enriched_correct/$tested = $(echo "scale=1%; $enriched_acc * 100" | bc)%"
echo "Absolute improvement: +$improvement_pct%"
echo ""
echo "Improvements: $improvements/$tested"
echo "Regressions:  $regressions/$tested"
echo ""

# Breakdown by conviction tier
echo "================================================================================"
echo "BREAKDOWN BY CONVICTION TIER"
echo "================================================================================"

for tier in strong mixed weak; do
    tier_tokens=$(jq -s -r ".[] | select(.tier == \"$tier\") | .symbol" "$results_file" | wc -l)
    if [ $tier_tokens -eq 0 ]; then
        continue
    fi

    tier_baseline=$(jq -s -r ".[] | select(.tier == \"$tier\") | select(.baseline_match == true)" "$results_file" | wc -l)
    tier_enriched=$(jq -s -r ".[] | select(.tier == \"$tier\") | select(.enriched_match == true)" "$results_file" | wc -l)
    tier_improvements=$(jq -s -r ".[] | select(.tier == \"$tier\") | select(.improvement == true)" "$results_file" | wc -l)

    tier_baseline_pct=$(echo "scale=1%; $tier_baseline / $tier_tokens * 100" | bc)
    tier_enriched_pct=$(echo "scale=1%; $tier_enriched / $tier_tokens * 100" | bc)

    echo ""
    echo "$(echo "$tier" | tr '[:lower:]' '[:upper:]') ($tier_tokens tokens):"
    echo "  Baseline: $tier_baseline/$tier_tokens = $tier_baseline_pct%"
    echo "  Enriched: $tier_enriched/$tier_tokens = $tier_enriched_pct%"
    echo "  Improvements: $tier_improvements"
done

echo ""
echo "================================================================================"
echo "RESULTS SAVED TO: $results_file"
echo "================================================================================"

# Hypothesis test
echo ""
echo "HYPOTHESIS TEST"
echo "================================================================================"

if [ $(echo "$enriched_acc > $baseline_acc" | bc) -eq 1 ]; then
    echo -e "${GREEN}✓ HYPOTHESIS SUPPORTED${NC}"
    echo "  Divergence enrichment improved accuracy by +$improvement_pct%"
    echo "  From $(echo "scale=1%; $baseline_acc * 100" | bc)% baseline to $(echo "scale=1%; $enriched_acc * 100" | bc)% enriched"
elif [ $(echo "$enriched_acc == $baseline_acc" | bc) -eq 1 ]; then
    echo -e "${YELLOW}⊘ INCONCLUSIVE${NC}"
    echo "  No statistically significant difference"
else
    echo -e "${RED}✗ HYPOTHESIS REJECTED${NC}"
    echo "  Divergence enrichment decreased accuracy"
fi
