#!/usr/bin/env bash
# qualify-dog.sh — Pre-deployment qualification for a CYNIC Dog endpoint.
# Runs 3 standardized stimuli and checks: valid JSON, all 6 axiom keys,
# scores in [0.0, 1.0], and latency within timeout.
#
# Usage: qualify-dog.sh <base_url> [api_key]
# Example: qualify-dog.sh http://localhost:8080/v1 mykey123
set -euo pipefail

BASE_URL="${1:-}"
AUTH_TOKEN="${2:-}"
TIMEOUT=120

if [[ -z "$BASE_URL" ]]; then
    echo "Usage: $(basename "$0") <base_url> [api_key]" >&2
    exit 1
fi

ENDPOINT="${BASE_URL}/chat/completions"

SYSTEM_PROMPT='You are a scoring engine. Given any input, respond ONLY with a single JSON object containing exactly these six keys: fidelity, phi, verify, culture, burn, sovereignty. Each value must be a float in [0.0, 1.0]. No prose, no markdown, no code fences — raw JSON only.'

STIMULI=(
    "Token RUGMASTER launched 2h ago. 98% held by top wallet. Liquidity pulled."
    "Jupiter Exchange (JUP) — leading Solana DEX. 2.4B+ volume, DAO governance, team doxxed."
    "1. e4 c5 — The Sicilian Defense. Black fights for the center asymmetrically."
)

LABELS=(
    "Stimulus 1 (rug — expect low)"
    "Stimulus 2 (legit DEX — expect high)"
    "Stimulus 3 (chess — expect moderate-high)"
)

AXIOMS=(fidelity phi verify culture burn sovereignty)

PASS=0
FAIL=0

auth_header() {
    if [[ -n "$AUTH_TOKEN" ]]; then
        echo "-H" "Authorization: Bearer ${AUTH_TOKEN}"
    fi
}

qualify_stimulus() {
    local idx="$1"
    local stimulus="${STIMULI[$idx]}"
    local label="${LABELS[$idx]}"

    local body
    body=$(jq -n \
        --arg sys "$SYSTEM_PROMPT" \
        --arg usr "$stimulus" \
        '{model:"default",messages:[{role:"system",content:$sys},{role:"user",content:$usr}],temperature:0}')

    local start_ms end_ms latency_ms raw content
    start_ms=$(date +%s%3N)

    # shellcheck disable=SC2046
    raw=$(curl -s --max-time "$TIMEOUT" \
        -X POST "$ENDPOINT" \
        -H "Content-Type: application/json" \
        $(auth_header) \
        -d "$body" 2>/dev/null) || true

    end_ms=$(date +%s%3N)
    latency_ms=$(( end_ms - start_ms ))

    content=$(echo "$raw" | jq -r '.choices[0].message.content // empty' 2>/dev/null || true)
    content=$(echo "$content" | sed 's/^```[a-z]*//;s/```$//' | tr -d '\n' | xargs)

    local json_status="OK"
    local score_status="OK"
    local missing_keys=()
    local bad_scores=()
    local parsed

    if ! parsed=$(echo "$content" | jq '.' 2>/dev/null); then
        json_status="FAIL (unparseable)"
        score_status="SKIP"
    else
        for key in "${AXIOMS[@]}"; do
            val=$(echo "$parsed" | jq --arg k "$key" '.[$k] // "MISSING"' 2>/dev/null)
            if [[ "$val" == '"MISSING"' ]]; then
                missing_keys+=("$key")
            fi
        done
        if [[ ${#missing_keys[@]} -gt 0 ]]; then
            json_status="FAIL (missing: ${missing_keys[*]})"
        fi

        if [[ ${#missing_keys[@]} -eq 0 ]]; then
            for key in "${AXIOMS[@]}"; do
                val=$(echo "$parsed" | jq --arg k "$key" '.[$k]' 2>/dev/null)
                ok=$(echo "$val" | awk '{if ($1+0 >= 0 && $1+0 <= 1) print 1; else print 0}')
                if [[ "$ok" != "1" ]]; then
                    bad_scores+=("${key}=${val}")
                fi
            done
            if [[ ${#bad_scores[@]} -gt 0 ]]; then
                score_status="FAIL (out-of-range: ${bad_scores[*]})"
            fi
        fi
    fi

    local result="PASS"
    if [[ "$json_status" != "OK" || "$score_status" != "OK" || "$score_status" == "SKIP" ]]; then
        result="FAIL"
        (( FAIL++ )) || true
    else
        (( PASS++ )) || true
    fi

    printf '[%s] %s: latency=%dms json=%s scores=%s\n' \
        "$result" "$label" "$latency_ms" "$json_status" "$score_status"
}

for i in 0 1 2; do
    qualify_stimulus "$i"
done

TOTAL=$(( PASS + FAIL ))
if [[ $FAIL -eq 0 ]]; then
    echo "RESULT: ${PASS}/${TOTAL} PASS — Dog QUALIFIED"
else
    echo "RESULT: ${PASS}/${TOTAL} PASS — Dog NOT qualified (need ${TOTAL}/${TOTAL})"
fi
