#!/bin/bash
# Phase 4: Demo Script — Kernel Health → /judge → Dogs → Verdict + Axioms
# Record this while running to show the fix in action

set -e

CYNIC_REST="${CYNIC_REST_ADDR:-http://localhost:3030}"
CYNIC_KEY="${CYNIC_API_KEY}"

echo "=================================================================="
echo "CYNIC Phase 4: Demo — Weak Dogs Produce BARK, No Silent Failures"
echo "=================================================================="
echo ""

# Step 1: Kernel Health (auth required — T1)
echo "[STEP 1] Kernel Health Check"
echo "Command: curl -s -H 'Authorization: Bearer \$CYNIC_API_KEY' \$CYNIC_REST/health | jq"
echo ""
curl -s -H "Authorization: Bearer ${CYNIC_API_KEY}" "$CYNIC_REST/health" | python3 -m json.tool
echo ""
echo "← Kernel is running. Dogs online or degraded."
echo ""

# Step 2: Weak Content Submission
echo "[STEP 2] Submit Weak Token Content to /judge"
echo "Command: curl -X POST \$CYNIC_REST/judge \\"
echo "  -H \"Authorization: Bearer \$CYNIC_API_KEY\" \\"
echo "  -H \"Content-Type: application/json\" \\"
echo "  -d '{\"content\":\"Check \$WIF token, seems weak\",\"domain\":\"token-analysis\"}'"
echo ""

RESPONSE=$(curl -s -X POST "$CYNIC_REST/judge" \
  -H "Authorization: Bearer $CYNIC_KEY" \
  -H "Content-Type: application/json" \
  -d '{"content":"WIF token analysis: low confidence signal, weak fundamentals","domain":"token-analysis"}')

echo "$RESPONSE" | python3 -m json.tool
echo ""

# Extract key fields
VERDICT=$(echo "$RESPONSE" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('verdict', 'ERROR'))" 2>/dev/null || echo "ERROR")
Q_SCORE=$(echo "$RESPONSE" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('q_score', {}).get('total', 0))" 2>/dev/null || echo "0")

echo "[STEP 3] Verdict Interpretation"
echo "Verdict: $VERDICT"
echo "Q-Score: $Q_SCORE (≤0.236 = BARK, low confidence)"
echo ""

if [ "$VERDICT" = "Bark" ]; then
    echo "✓ VALIDATION: Weak Dogs produce BARK (audible signal)"
    echo "  Before fix: DegenerateScores error → 500 error (silent failure)"
    echo "  After fix:  Weak Dogs → BARK verdict (honest weak judgment)"
    echo ""
else
    echo "✗ Unexpected verdict: $VERDICT"
fi

echo "[STEP 4] Axiom Breakdown"
echo "The BARK verdict shows Dogs are honest about their confusion:"
echo ""
python3 << EOF
import json
response = '''$RESPONSE'''
try:
    data = json.loads(response)
    if 'q_score' in data:
        qs = data['q_score']
        print(f"  Fidelity:    {qs.get('fidelity', 0):.3f} (faithful to facts)")
        print(f"  Phi:         {qs.get('phi', 0):.3f} (structural harmony)")
        print(f"  Verify:      {qs.get('verify', 0):.3f} (testable/verifiable)")
        print(f"  Culture:     {qs.get('culture', 0):.3f} (cultural respect)")
        print(f"  Burn:        {qs.get('burn', 0):.3f} (efficiency)")
        print(f"  Sovereignty: {qs.get('sovereignty', 0):.3f} (agency)")
except Exception as e:
    print(f"  (Axiom data parsing error: {e})")
EOF

echo ""
echo "[COMPLETE] Demo shows: weak Dogs communicate as BARK, not errors"
echo "=================================================================="
echo ""
echo "For submission: Record this demo showing kernel→judge→verdict→axioms"
echo "Upload to hackathon with Phase 2/3 reports as evidence"
