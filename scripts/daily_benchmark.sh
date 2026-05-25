#!/usr/bin/env bash
# Daily Token Benchmark — judge 45 tracked tokens, measure discrimination.
#
# Runs daily via systemd (06:20 UTC). Produces JSONL with verdicts + enrichment
# snapshots. Computes discrimination metric and alerts on drift.
#
# Category mapping:
#   watchlist.json source → blue_chip/defi/infra=LEGIT, low_cap=SURVIVOR,
#                           mid_meme/ai_agent=SKETCHY
#   calibration_results_real.json conviction_tier → strong=LEGIT, mixed=SURVIVOR, weak=DEAD
#
# K15 consumer: drift-alert observation when discrimination < 0.
# K25: 2s sleep between /judge calls to avoid slot starvation.
#
# Usage:
#   ./scripts/daily_benchmark.sh              # run benchmark
#   ./scripts/daily_benchmark.sh --report     # print 7d discrimination trend
#
# Requires: CYNIC_REST_ADDR, CYNIC_API_KEY (from ~/.cynic-env), jq

set -euo pipefail

# ── ENV ──
for env_file in ~/.cynic-env ~/.config/cynic/env; do
    # shellcheck source=/dev/null
    [ -f "$env_file" ] && source "$env_file"
done

CYNIC_REST_ADDR="${CYNIC_REST_ADDR:?CYNIC_REST_ADDR not set}"
CYNIC_API_KEY="${CYNIC_API_KEY:?CYNIC_API_KEY not set}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WATCHLIST="${PROJECT_DIR}/cynic-python/heuristics/collection/watchlist.json"
CALIB="${PROJECT_DIR}/cynic-python/heuristics/data/calibration_results_real.json"
OUTPUT_DIR="${PROJECT_DIR}/cynic-python/data/benchmark_daily"
TODAY="$(date -u +%Y-%m-%d)"
OUTPUT_FILE="${OUTPUT_DIR}/${TODAY}.jsonl"

mkdir -p "$OUTPUT_DIR"

# ── Report mode ──
if [ "${1:-}" = "--report" ]; then
    echo "=== Discrimination Trend (last 7 days) ==="
    for f in $(ls -1 "$OUTPUT_DIR"/*.jsonl 2>/dev/null | sort | tail -7); do
        day=$(basename "$f" .jsonl)
        disc=$(jq -s '[.[] | select(.category == "LEGIT") | .q_score] | min' "$f" 2>/dev/null || echo "null")
        dead_max=$(jq -s '[.[] | select(.category == "DEAD") | .q_score] | max' "$f" 2>/dev/null || echo "null")
        if [ "$disc" != "null" ] && [ "$dead_max" != "null" ]; then
            delta=$(echo "$disc - $dead_max" | bc -l 2>/dev/null || echo "?")
            printf "  %s  discrimination: %s  (LEGIT min: %s, DEAD max: %s)\n" "$day" "$delta" "$disc" "$dead_max"
        else
            printf "  %s  insufficient data\n" "$day"
        fi
    done
    exit 0
fi

# ── Build token list with categories ──
# Merge watchlist.json (source-based categories) + calibration_results_real.json (conviction-based)
# Deduplicate by mint address.

build_token_list() {
    local tokens="[]"

    # Watchlist: map source → category
    if [ -f "$WATCHLIST" ]; then
        tokens=$(jq '
            [.[] | {
                mint: .mint,
                symbol: .symbol,
                category: (
                    if .source == "blue_chip" or .source == "defi" or .source == "infra" then "LEGIT"
                    elif .source == "low_cap" then "SURVIVOR"
                    elif .source == "mid_meme" or .source == "ai_agent" then "SKETCHY"
                    else "UNKNOWN"
                    end
                )
            }]
        ' "$WATCHLIST")
    fi

    # Calibration: map conviction_tier → category
    if [ -f "$CALIB" ]; then
        local calib_tokens
        calib_tokens=$(jq '
            [.results[] | {
                mint: .mint,
                symbol: .symbol,
                category: (
                    if .conviction_tier == "strong" then "LEGIT"
                    elif .conviction_tier == "mixed" then "SURVIVOR"
                    elif .conviction_tier == "weak" then "DEAD"
                    else "UNKNOWN"
                    end
                )
            }]
        ' "$CALIB")

        # Merge and deduplicate (watchlist takes priority for overlapping mints)
        local existing_mints
        existing_mints=$(echo "$tokens" | jq -r '.[].mint')
        tokens=$(echo "$tokens" "$calib_tokens" | jq -s '
            .[0] as $existing |
            ($existing | [.[].mint]) as $seen |
            $existing + [.[1][] | select(.mint as $m | $seen | index($m) | not)]
        ')
    fi

    echo "$tokens"
}

TOKENS=$(build_token_list)
TOKEN_COUNT=$(echo "$TOKENS" | jq 'length')

echo "=== CYNIC Daily Token Benchmark ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Tokens: ${TOKEN_COUNT}"
echo "Output: ${OUTPUT_FILE}"
echo ""

# ── Judge each token ──
SUCCESS_COUNT=0
FAIL_COUNT=0

for i in $(seq 0 $((TOKEN_COUNT - 1))); do
    MINT=$(echo "$TOKENS" | jq -r ".[$i].mint")
    SYMBOL=$(echo "$TOKENS" | jq -r ".[$i].symbol")
    CATEGORY=$(echo "$TOKENS" | jq -r ".[$i].category")

    printf "  [%d/%d] %s (%s)... " "$((i + 1))" "$TOKEN_COUNT" "$SYMBOL" "$CATEGORY"

    # POST /judge with retry on transient errors (P18)
    RESPONSE=""
    HTTP_CODE=""
    for attempt in 1 2; do
        HTTP_RESULT=$(timeout 120 curl -s -w "\n%{http_code}" --max-time 115 \
            "http://${CYNIC_REST_ADDR}/judge" \
            -H "Authorization: Bearer ${CYNIC_API_KEY}" \
            -H "Content-Type: application/json" \
            -d "{\"content\": \"${MINT}\", \"domain\": \"token-analysis\"}" 2>/dev/null || echo -e "\n000")

        HTTP_CODE=$(echo "$HTTP_RESULT" | tail -1)
        RESPONSE=$(echo "$HTTP_RESULT" | sed '$d')

        if [ "$HTTP_CODE" = "200" ]; then
            break
        elif [ "$HTTP_CODE" = "429" ] || [ "$HTTP_CODE" = "503" ]; then
            if [ "$attempt" = "1" ]; then
                echo -n "retry(${HTTP_CODE})... "
                sleep 10
            fi
        else
            # Permanent error (400/404/422) or connection failure — don't retry
            break
        fi
    done

    if [ "$HTTP_CODE" != "200" ] || [ -z "$RESPONSE" ]; then
        echo "FAIL (HTTP ${HTTP_CODE})"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        sleep 2
        continue
    fi

    # Parse response with jq
    Q_SCORE=$(echo "$RESPONSE" | jq -r '.q_score.total // 0')
    VERDICT=$(echo "$RESPONSE" | jq -r '.verdict // "error"')
    AXIOM_SCORES=$(echo "$RESPONSE" | jq -c '{
        fidelity: (.q_score.fidelity // 0),
        phi: (.q_score.phi // 0),
        verify: (.q_score.verify // 0),
        culture: (.q_score.culture // 0),
        burn: (.q_score.burn // 0),
        sovereignty: (.q_score.sovereignty // 0)
    }')
    DOG_SCORES=$(echo "$RESPONSE" | jq -c '
        if .dog_scores then
            [.dog_scores[] | {(.dog_name // .dog_id // "unknown"): {q: .q_score}}] | add // {}
        else {}
        end
    ')
    ENRICHMENT=$(echo "$RESPONSE" | jq -c '{
        holders: (.token_data.holder_count // null),
        top_1_pct: (.token_data.top_1_wallet_pct // null),
        herfindahl: (.token_data.herfindahl_index // null),
        age_hours: (.token_data.age_hours // null),
        k_score: (.token_data.k_score // null),
        diamond_hands: (.token_data.diamond_hands // null),
        longevity: (.token_data.longevity // null),
        organic_growth: (.token_data.organic_growth // null),
        supply_burned_pct: (.token_data.supply_burned_pct // null),
        liquidity_usd: (.token_data.liquidity_usd // null),
        volume_24h_usd: (.token_data.volume_24h_usd // null),
        effective_concentration: (.token_data.effective_wallet_concentration // null),
        trajectory_class: (.token_data.trajectory_class // null),
        accumulators: (.token_data.accumulators // null),
        extractors: (.token_data.extractors // null)
    }')

    echo "${VERDICT} Q=${Q_SCORE}"

    # Write JSONL row (P17: schema_version=1)
    jq -n -c \
        --arg date "$TODAY" \
        --arg mint "$MINT" \
        --arg symbol "$SYMBOL" \
        --arg category "$CATEGORY" \
        --argjson q_score "$Q_SCORE" \
        --arg verdict "$VERDICT" \
        --argjson axiom_scores "$AXIOM_SCORES" \
        --argjson dog_scores "$DOG_SCORES" \
        --argjson enrichment "$ENRICHMENT" \
        '{
            date: $date,
            mint: $mint,
            symbol: $symbol,
            category: $category,
            q_score: $q_score,
            verdict: $verdict,
            axiom_scores: $axiom_scores,
            dog_scores: $dog_scores,
            enrichment_snapshot: $enrichment,
            schema_version: 1
        }' >> "$OUTPUT_FILE"

    SUCCESS_COUNT=$((SUCCESS_COUNT + 1))

    # K25: rate-limit between calls
    sleep 2
done

echo ""
echo "=== Results ==="
echo "Success: ${SUCCESS_COUNT}/${TOKEN_COUNT}, Failed: ${FAIL_COUNT}"

# ── Compute discrimination ──
if [ "$SUCCESS_COUNT" -lt $((TOKEN_COUNT / 2)) ]; then
    echo "WARNING: <50% success rate — discrimination not computed"
    # Post warning observation
    timeout 10 curl -s --max-time 8 -X POST "http://${CYNIC_REST_ADDR}/observe" \
        -H "Authorization: Bearer ${CYNIC_API_KEY}" \
        -H "Content-Type: application/json" \
        -d "{\"tool\":\"daily_benchmark\",\"target\":\"benchmark\",\"domain\":\"benchmark\",\"context\":\"WARNING: only ${SUCCESS_COUNT}/${TOKEN_COUNT} tokens judged — sample too small for discrimination\",\"tags\":[\"benchmark-warning\",\"no-rejudge\"]}" \
        >/dev/null 2>&1 || true
    exit 0
fi

LEGIT_MIN=$(jq -s '[.[] | select(.category == "LEGIT") | .q_score] | if length > 0 then min else null end' "$OUTPUT_FILE")
DEAD_MAX=$(jq -s '[.[] | select(.category == "DEAD") | .q_score] | if length > 0 then max else null end' "$OUTPUT_FILE")

if [ "$LEGIT_MIN" != "null" ] && [ "$DEAD_MAX" != "null" ]; then
    DISCRIMINATION=$(echo "$LEGIT_MIN - $DEAD_MAX" | bc -l)
    echo "Discrimination (min LEGIT - max DEAD): ${DISCRIMINATION}"

    # Append discrimination to each row retroactively? No — just report.
    if echo "$DISCRIMINATION < 0" | bc -l | grep -q "1"; then
        echo "ALERT: Negative discrimination — LEGIT/DEAD overlap detected"
        # K21: post alert with no-rejudge tag to prevent compound-loop
        timeout 10 curl -s --max-time 8 -X POST "http://${CYNIC_REST_ADDR}/observe" \
            -H "Authorization: Bearer ${CYNIC_API_KEY}" \
            -H "Content-Type: application/json" \
            -d "{\"tool\":\"daily_benchmark\",\"target\":\"discrimination\",\"domain\":\"benchmark\",\"context\":\"DRIFT ALERT: discrimination=${DISCRIMINATION} (LEGIT min=${LEGIT_MIN}, DEAD max=${DEAD_MAX}). Token scoring is not separating legitimate from dead tokens.\",\"tags\":[\"drift-alert\",\"no-rejudge\"]}" \
            >/dev/null 2>&1 || true
    else
        echo "Dogs discriminate: LEGIT scores above DEAD"
    fi
else
    echo "Note: insufficient LEGIT or DEAD tokens for discrimination metric"
fi

# Per-category summary
echo ""
echo "=== Per-Category Summary ==="
jq -s '
    group_by(.category) | .[] |
    {
        category: .[0].category,
        count: length,
        mean_q: ([.[].q_score] | add / length),
        min_q: ([.[].q_score] | min),
        max_q: ([.[].q_score] | max),
        verdicts: [.[].verdict] | group_by(.) | map({(.[0]): length}) | add
    }
' "$OUTPUT_FILE" | jq -r '
    "\(.category)\t n=\(.count)\t Q: \(.min_q | tostring | .[:5])-\(.max_q | tostring | .[:5]) (mean \(.mean_q | tostring | .[:5]))\t \(.verdicts)"
'

echo ""
echo "Output: ${OUTPUT_FILE}"
