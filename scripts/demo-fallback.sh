#!/bin/bash
# CYNIC Demo Fallback — terminal demo if frontend isn't ready
# Usage: bash scripts/demo-fallback.sh [BASE_URL]

BASE="${1:-http://localhost:3030}"
BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
GOLD='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

format_verdict() {
    local json="$1"
    local verdict=$(echo "$json" | python3 -c "import sys,json;print(json.load(sys.stdin)['verdict'])")
    local total=$(echo "$json" | python3 -c "import sys,json;print(f\"{json.load(sys.stdin)['q_score']['total']:.3f}\")")
    local dogs=$(echo "$json" | python3 -c "import sys,json;print(json.load(sys.stdin)['dogs_used'])")
    local anomaly=$(echo "$json" | python3 -c "import sys,json;r=json.load(sys.stdin);print(f\"ANOMALY on {r['anomaly_axiom']}\" if r['anomaly_detected'] else 'consensus')")

    case "$verdict" in
        Howl) color=$GOLD ;;
        Wag)  color=$BLUE ;;
        Growl) color=$GOLD ;;
        Bark) color=$RED ;;
        *)    color=$NC ;;
    esac

    echo -e "${BOLD}Verdict: ${color}${verdict}${NC}  Q-Score: ${BOLD}${total}${NC}/0.618  [${dogs}]  ${anomaly}"
    echo ""

    # Axiom scores
    echo "$json" | python3 -c "
import sys,json
r=json.load(sys.stdin)
q=r['q_score']
axioms=['fidelity','phi','verify','culture','burn','sovereignty']
icons={'fidelity':'🛡️','phi':'🌀','verify':'🔍','culture':'🏛️','burn':'🔥','sovereignty':'👑'}
for a in axioms:
    score=q[a]
    bar='█' * int(score / 0.618 * 20) + '░' * (20 - int(score / 0.618 * 20))
    print(f'  {icons[a]} {a.upper():12s} [{bar}] {score:.3f}')
print()
# Reasoning (first Dog with non-heuristic reasoning)
for d in r.get('dog_scores',[]):
    if 'Heuristic' not in d['reasoning'].get('fidelity',''):
        print('  Reasoning:')
        for a in axioms:
            reason = d['reasoning'].get(a,'')[:80]
            print(f'    {a}: {reason}')
        break
"
}

echo -e "${BOLD}╔══════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║  CYNIC — Epistemic Immune System — Live Demo        ║${NC}"
echo -e "${BOLD}║  φ-bounded confidence: max 61.8%                    ║${NC}"
echo -e "${BOLD}╚══════════════════════════════════════════════════════╝${NC}"
echo ""

# Health check
echo -e "${BOLD}[1/4] System Health${NC}"
health=$(curl -s "$BASE/health")
echo "$health" | python3 -c "
import sys,json
r=json.load(sys.stdin)
print(f\"  Status: {r['status']}  |  Axioms: {', '.join(r['axioms'])}  |  φ_max: {r['phi_max']}\")
"
echo ""

# Demo 1: Fool's Mate (BARK)
echo -e "${BOLD}[2/4] Fool's Mate — the WORST opening in chess${NC}"
echo "  1. f3 e5 2. g4 Qh4# — checkmate in 2 moves"
echo ""
result=$(curl -s -X POST "$BASE/judge" \
    -H "Content-Type: application/json" \
    -d '{"content":"1. f3 e5 2. g4 Qh4# — Fools Mate. White walks into checkmate in 2 moves.","context":"Worst opening in chess — no grandmaster would ever play this","domain":"chess"}')
format_verdict "$result"
echo ""
read -p "  [Press Enter for next demo...]"
echo ""

# Demo 2: Sicilian Defense (HOWL)
echo -e "${BOLD}[3/4] Sicilian Defense — the BEST response to 1.e4${NC}"
echo "  1. e4 c5 — asymmetric fight for the center"
echo ""
result=$(curl -s -X POST "$BASE/judge" \
    -H "Content-Type: application/json" \
    -d '{"content":"1. e4 c5 — The Sicilian Defense. Black fights for the center asymmetrically, creating imbalanced positions rich in possibilities.","context":"Most popular and successful response to 1.e4 at grandmaster level","domain":"chess"}')
format_verdict "$result"
echo ""
read -p "  [Press Enter for next demo...]"
echo ""

# Demo 3: Free text — flat earth (BARK)
echo -e "${BOLD}[4/4] Universal — same axioms, any domain${NC}"
echo "  Claim: \"The earth is flat\""
echo ""
result=$(curl -s -X POST "$BASE/judge" \
    -H "Content-Type: application/json" \
    -d '{"content":"The earth is flat.","context":"Scientific claim made by flat earth conspiracy theorists","domain":"science"}')
format_verdict "$result"
echo ""
echo -e "${BOLD}═══════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  3 independent AIs. 6 philosophical axioms.${NC}"
echo -e "${BOLD}  Maximum confidence: 61.8% — epistemic humility by design.${NC}"
echo -e "${BOLD}  This is CYNIC — an epistemic immune system.${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════════${NC}"
