#!/usr/bin/env bash
# Hermes Organ X — K15 E2E Smoke Test
# Injects a synthetic observation, verifies it traverses the full pipeline.
#
# Prerequisites:
#   - cynic-kernel running (curl /health returns 200)
#   - hermes-agent-executor running or invoked manually
#   - CYNIC_REST_ADDR and CYNIC_API_KEY set
#
# Usage: source ~/.cynic-env && bash test_executor_smoke.sh

set -euo pipefail

KERNEL="${CYNIC_REST_ADDR:?CYNIC_REST_ADDR not set}"
API_KEY="${CYNIC_API_KEY:?CYNIC_API_KEY not set}"
AUTH="Authorization: Bearer ${API_KEY}"

# Ensure http:// prefix
[[ "$KERNEL" == http* ]] || KERNEL="http://${KERNEL}"

echo "=== Step 1: Inject synthetic high-signal observation ==="
OBS_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "${KERNEL}/observe" \
  -H "${AUTH}" -H "Content-Type: application/json" \
  -d '{
    "tool": "smoke_test",
    "target": "SMOKE_TEST_TOKEN_'"$(date +%s)"'",
    "domain": "twitter",
    "context": "E2E smoke test: synthetic high-signal observation for K15 pipeline validation",
    "agent_id": "smoke-test",
    "tags": ["smoke-test", "high-signal"]
  }')
HTTP_CODE=$(echo "$OBS_RESPONSE" | tail -1)
echo "POST /observe: HTTP ${HTTP_CODE}"
if [ "$HTTP_CODE" != "200" ] && [ "$HTTP_CODE" != "201" ]; then
  echo "FAIL: Could not inject observation"
  exit 1
fi
echo "OK: Observation injected"

echo ""
echo "=== Step 2: Check pending agent tasks ==="
TASKS=$(curl -s -H "${AUTH}" "${KERNEL}/agent-tasks?status=pending")
echo "Pending tasks: ${TASKS}"

echo ""
echo "=== Step 3: Verify executor service is running ==="
systemctl --user is-active hermes-agent-executor.service && echo "OK: Executor running" || echo "WARN: Executor not running — run manually for test"

echo ""
echo "=== Step 4: Check recent observations ==="
echo "Waiting 10s for pipeline processing..."
sleep 10
RECENT_OBS=$(curl -s -H "${AUTH}" "${KERNEL}/observations?domain=twitter&limit=3")
echo "Recent observations: ${RECENT_OBS}" | head -5

echo ""
echo "=== Smoke test complete ==="
echo "Manual verification needed:"
echo "  1. Check executor logs: journalctl --user -eu hermes-agent-executor | tail -20"
echo "  2. Check K15 consumer: journalctl --user -eu hermes-k15-consumer | tail -20"
echo "  3. Check verdicts: curl -s -H \"Authorization: Bearer \${CYNIC_API_KEY}\" \"http://\${CYNIC_REST_ADDR}/verdicts?limit=3\""
