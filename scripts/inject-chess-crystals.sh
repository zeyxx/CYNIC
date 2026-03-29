#!/usr/bin/env bash
# inject-chess-crystals.sh — Seed chess evaluation crystals for LLM Dogs.
#
# Design derived from ML/DL first principles:
#
# BUDGET: domain_prompt × φ⁻² = 2850 × 0.382 = 1089 chars
#   Three independent derivations converge (golden ratio weighting, attention
#   research, few-shot ICL) at ~1100 chars. Pipeline uses 1100.
#
# CONTENT LENGTH: L* ≈ 178 chars (max 203)
#   From log-diminishing-returns optimization: V = n × log(L/50),
#   subject to n × (55+L) ≤ 1032. Fibonacci verification: 178 = F(12)+F(9).
#   Yields n*=4 crystals per injection.
#
# ORDERING: positive-first (primacy), anti-patterns last (recency)
#   "Lost in the middle" (arXiv 2307.03172): first/last positions get 30%+
#   more attention. Positive primacy counters LLM positivity-then-negativity bias.
#   Anti-pattern recency provides final calibration before scoring.
#
# SIGNAL SEPARATION: positive and negative signals in SEPARATE crystals.
#   Mixing "excellent" and "poor" in one crystal causes LLM negativity
#   contamination — the negative signal dominates.
#
# DEEP BENCH (crystals 5-8): lower confidence, not injected by default.
#   Retrieved via semantic KNN for specific stimuli that match their content.

set -euo pipefail
export LC_NUMERIC=C
source ~/.cynic-env 2>/dev/null || true

SURREAL_URL="${SURREALDB_URL:-http://localhost:8000}"
SURREAL_NS="${SURREALDB_NS:-cynic}"
SURREAL_DB="${SURREALDB_DB:-v2}"
SURREAL_USER="${SURREALDB_USER:-root}"
SURREAL_PASS="${SURREALDB_PASS:?SURREALDB_PASS must be set}"

AUTH=$(echo -n "${SURREAL_USER}:${SURREAL_PASS}" | base64)
TS=$(date -u +%Y-%m-%dT%H:%M:%SZ)

sql() {
    curl -sf -X POST "${SURREAL_URL}/sql" \
        -H "Accept: application/json" \
        -H "surreal-ns: ${SURREAL_NS}" \
        -H "surreal-db: ${SURREAL_DB}" \
        -H "Authorization: Basic ${AUTH}" \
        --data-raw "$1"
}

inject() {
    local id="$1" conf="$2" content="$3"
    content="${content//\'/\\\'}"
    sql "UPSERT crystal:\`${id}\` SET content='${content}', domain='chess', observations=25, confidence=${conf}, state='crystallized', created_at='${TS}', updated_at='${TS}';" > /dev/null
    printf "  %-38s conf=%.2f  %3d chars\n" "$id" "$conf" "${#content}"
}

echo "▶ Injecting chess evaluation crystals"
echo "  Budget: 1100 chars (domain_prompt × φ⁻²)"
echo "  Design: 4 primary (injected) + 4 bench (KNN retrieval)"
echo

# Clean slate
sql "DELETE crystal WHERE domain = 'chess';" > /dev/null
echo "  Cleaned existing chess crystals"
echo

# ── PRIMARY (4 crystals, injected per query) ──────────────────────
# Order: positive(primacy) → calibration → anti-pattern → anti-pattern(recency)

echo "── Primary (injected per query) ──"

inject "chess_c3_pos_endgame_win" 0.95 \
"Winning endgame technique — opposition, triangulation, Lucena bridge, passed pawn conversion — is among the deepest chess. Pure calculation and geometry. High VERIFY, high PHI, high SOVEREIGNTY. This is mastery."

inject "chess_c2_cal_tiers" 0.93 \
"HOWL: Sicilian, King's Indian, winning endgame technique. WAG: sound lines (QGD, Berlin). GROWL: dubious play, drawn-despite-advantage (wrong-colored bishop, fortress = wasted advantage). BARK: Fool's Mate, aimless moves."

inject "chess_c1_anti_bad_is_bad" 0.91 \
"Bad chess is bad. Do not find positives in poor play. Premature queen moves are weak, not bold. Aimless shuffling is planless, not flexible. Multiple principle violations = LOW across all axioms."

inject "chess_c4_anti_hope" 0.89 \
"Hope chess — moves that only work if the opponent blunders — is unsound. Judge by best reply. Sacrifices without compensation, one-move threats that lose if parried, stalemate without resources = hope chess."

# ── DEEP BENCH (semantic KNN retrieval) ───────────────────────────

echo "── Deep bench (KNN retrieval) ──"

inject "chess_c5_center_develop" 0.78 \
"Center control + rapid development are fundamental. Pieces active by move 8, castling by 10 = high FIDELITY+BURN. Hypermodern openings concede center to attack later — valid advanced strategy, not a violation."

inject "chess_c6_tempo_value" 0.76 \
"Every move is a resource. Same piece moved twice, random flank pushes, or pointless checks all waste tempi. Two wasted tempi ≈ one pawn disadvantage. Score BURN near zero for tempo waste."

inject "chess_c7_sacrifice_test" 0.74 \
"Sacrifices need concrete compensation: initiative, king attack, structure damage, forced tactics. With compensation = brilliant. Without = blunder. Test: does it work against best defense?"

inject "chess_c8_position_not_concept" 0.72 \
"Score position quality, not concept's educational value. A drawn endgame is poor even if famous. A blunder described well is still a blunder. Is this good CHESS, not is this interesting?"

echo
echo "── Verification ──"
resp=$(sql "SELECT id, confidence, observations, state FROM crystal WHERE domain = 'chess' ORDER BY confidence DESC;")
echo "$resp" | python3 -c "
import json, sys
data = json.load(sys.stdin)
crystals = data[0]['result'] if data else []
print(f'  Chess crystals: {len(crystals)}')
for c in crystals:
    cid = c.get('id', '?')
    if isinstance(cid, dict): cid = list(cid.values())[-1]
    print(f'    {c.get(\"confidence\",0):.2f} [{c.get(\"state\",\"?\"):>13s}] {cid}')
" 2>/dev/null || echo "  (parse failed)"

echo
echo "Done. Run: source ~/.cynic-env && python3 scripts/chess-bench.py 2"
