#!/usr/bin/env bash
# E2E Token Pipeline Benchmark
# Runs a curated set of tokens through /judge and measures accuracy + discrimination.
#
# Ground truth categories:
#   LEGIT  = established DeFi/stablecoin, should score WAG/HOWL
#   SURVIVOR = memecoin that survived >30 days with real activity
#   SKETCHY  = volatile/political token, should score GROWL
#   DEAD     = known dead/abandoned token, should score BARK/GROWL
#
# Usage: ./scripts/e2e-token-benchmark.sh
# Requires: CYNIC_REST_ADDR, CYNIC_API_KEY in ~/.cynic-env

set -euo pipefail
source ~/.cynic-env 2>/dev/null

RESULTS_FILE="/tmp/cynic-token-benchmark-$(date +%Y%m%d-%H%M%S).json"

# ── Curated test set ──
# Format: mint|category|label
TOKENS=(
  # LEGIT — established, high liquidity, real utility
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v|LEGIT|USDC"
  "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB|LEGIT|USDT"
  "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R|LEGIT|RAY"
  "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN|LEGIT|JUP"
  "jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL|LEGIT|JTO"
  # SURVIVOR — memecoins that survived with real community
  "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263|SURVIVOR|BONK"
  "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm|SURVIVOR|WIF"
  "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr|SURVIVOR|POPCAT"
  # SKETCHY — political/celebrity, high manipulation risk
  "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN|SKETCHY|TRUMP"
  "HeLp6NuQkmYB4pYWo2zYs22mESHXPQYzXbB8n4V98jwC|SKETCHY|AI16Z"
  # DEAD — tokens with minimal/zero activity (pump.fun remnants)
  "FU1q8vJpZNUrmqsciSjp8bAKKidGsLmouB8CBdf8TKQv|DEAD|BALD"
  "CKfatsPMUf8SkiURsDXs7eK6GWb31BMtE1MpBhKpump|DEAD|PUMPFUN-DEAD1"
)

echo "=== CYNIC Token Pipeline Benchmark ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Tokens: ${#TOKENS[@]}"
echo "Output: ${RESULTS_FILE}"
echo ""

# Initialize results JSON array
echo '[' > "$RESULTS_FILE"
FIRST=true

for entry in "${TOKENS[@]}"; do
  IFS='|' read -r mint category label <<< "$entry"

  echo -n "  ${label} (${category})... "

  START_MS=$(date +%s%3N)

  RESPONSE=$(timeout 120 curl -s --max-time 115 "http://${CYNIC_REST_ADDR}/judge" \
    -H "Authorization: Bearer ${CYNIC_API_KEY}" \
    -H "Content-Type: application/json" \
    -d "{\"content\": \"${mint}\", \"domain\": \"token-analysis\"}" 2>/dev/null || echo '{"error": "timeout"}')

  END_MS=$(date +%s%3N)
  LATENCY=$((END_MS - START_MS))

  # Extract fields
  Q_SCORE=$(echo "$RESPONSE" | jq -r '.q_score.total // "null"')
  VERDICT=$(echo "$RESPONSE" | jq -r '.verdict // "error"')
  VOTERS=$(echo "$RESPONSE" | jq -r '.voter_count // 0')
  TOKEN_NAME=$(echo "$RESPONSE" | jq -r '.token_data.name // "null"')
  HOLDERS=$(echo "$RESPONSE" | jq -r '.token_data.holder_count // 0')
  VOLUME=$(echo "$RESPONSE" | jq -r '.token_data.volume_24h_usd // 0')
  LIQUIDITY=$(echo "$RESPONSE" | jq -r '.token_data.liquidity_usd // 0')
  ENRICHED=$(echo "$RESPONSE" | jq -r 'if .token_data.name then "yes" else "no" end')
  ERROR=$(echo "$RESPONSE" | jq -r '.error // "none"')

  echo "${VERDICT} Q=${Q_SCORE} (${LATENCY}ms, ${VOTERS} dogs, enriched=${ENRICHED})"

  # Append to results
  if [ "$FIRST" = true ]; then
    FIRST=false
  else
    echo ',' >> "$RESULTS_FILE"
  fi

  cat >> "$RESULTS_FILE" << JSONEOF
  {
    "mint": "${mint}",
    "label": "${label}",
    "category": "${category}",
    "verdict": "${VERDICT}",
    "q_score": ${Q_SCORE:-null},
    "voters": ${VOTERS},
    "latency_ms": ${LATENCY},
    "enriched": "${ENRICHED}",
    "token_name": "${TOKEN_NAME}",
    "holders": ${HOLDERS},
    "volume_24h": ${VOLUME},
    "liquidity": ${LIQUIDITY},
    "error": "${ERROR}"
  }
JSONEOF

  # K25: rate-limit between calls
  sleep 2
done

echo ']' >> "$RESULTS_FILE"

echo ""
echo "=== Results Analysis ==="
python3 -c "
import json, sys

with open('${RESULTS_FILE}') as f:
    results = json.load(f)

# Group by category
from collections import defaultdict
groups = defaultdict(list)
for r in results:
    groups[r['category']].append(r)

# Per-category stats
print(f'Total: {len(results)} tokens')
print()
print(f'{\"Category\":<12} {\"N\":>3} {\"Mean Q\":>8} {\"Min Q\":>8} {\"Max Q\":>8} {\"Verdicts\":<20} {\"Enriched\":>8}')
print('-' * 80)

for cat in ['LEGIT', 'SURVIVOR', 'SKETCHY', 'DEAD']:
    items = groups.get(cat, [])
    if not items:
        continue
    qs = [r['q_score'] for r in items if r['q_score'] is not None and r['q_score'] != 'null']
    if not qs:
        qs = [0]
    verdicts = [r['verdict'] for r in items]
    enriched = sum(1 for r in items if r['enriched'] == 'yes')
    verdict_dist = {}
    for v in verdicts:
        verdict_dist[v] = verdict_dist.get(v, 0) + 1
    vstr = ', '.join(f'{k}:{v}' for k, v in sorted(verdict_dist.items()))
    print(f'{cat:<12} {len(items):>3} {sum(qs)/len(qs):>8.3f} {min(qs):>8.3f} {max(qs):>8.3f} {vstr:<20} {enriched:>3}/{len(items)}')

# Discrimination check
print()
legit_qs = [r['q_score'] for r in groups.get('LEGIT', []) if isinstance(r['q_score'], (int, float))]
dead_qs = [r['q_score'] for r in groups.get('DEAD', []) if isinstance(r['q_score'], (int, float))]
if legit_qs and dead_qs:
    sep = min(legit_qs) - max(dead_qs)
    print(f'Discrimination (min LEGIT - max DEAD): {sep:+.3f}')
    if sep > 0:
        print('  -> Dogs discriminate: LEGIT scores strictly above DEAD')
    else:
        print('  -> WARNING: overlap between LEGIT and DEAD scores')

# Enrichment rate
enriched_count = sum(1 for r in results if r['enriched'] == 'yes')
print(f'Enrichment rate: {enriched_count}/{len(results)} ({100*enriched_count/len(results):.0f}%)')

# Latency
latencies = [r['latency_ms'] for r in results]
print(f'Latency: P50={sorted(latencies)[len(latencies)//2]}ms, max={max(latencies)}ms')

# Errors
errors = [r for r in results if r['error'] != 'none']
if errors:
    print(f'Errors: {len(errors)} — {[e[\"label\"] + \": \" + e[\"error\"] for e in errors]}')
else:
    print('Errors: 0')
"
