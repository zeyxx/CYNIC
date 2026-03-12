#!/bin/bash
set -euo pipefail

echo "╔══════════════════════════════════════╗"
echo "║       CYNIC DEMO — Hackathon MVP     ║"
echo "╚══════════════════════════════════════╝"

API="http://localhost:3000"

echo ""
echo "=== Health Check ==="
curl -s "$API/health" | python3 -m json.tool

echo ""
echo "=== Judge: Chess Move ==="
curl -s -X POST "$API/judge" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "e4 e5 Nf3 Nc6 Bb5",
    "context": "Ruy Lopez opening - a classical, well-studied opening",
    "domain": "chess"
  }' | python3 -m json.tool

echo ""
echo "=== Judge: Overconfident Claim ==="
curl -s -X POST "$API/judge" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "This trading strategy will always make money 100% guaranteed",
    "domain": "trading"
  }' | python3 -m json.tool

echo ""
echo "=== Judge: Honest Analysis ==="
curl -s -X POST "$API/judge" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Based on the evidence, this approach probably works in approximately 60% of cases because of X and Y factors",
    "context": "Analyzing market patterns with epistemic humility",
    "domain": "geopolitics"
  }' | python3 -m json.tool

echo ""
echo "=== Recent Verdicts ==="
curl -s "$API/verdicts" | python3 -m json.tool

echo ""
echo "Demo complete. Max confidence: phi^-1 = 0.618"
