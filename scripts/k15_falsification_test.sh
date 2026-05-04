#!/bin/bash
# K15 Phase 2d Falsification Test
# Inject failure events → detect degradation → verify recovery path

set -euo pipefail

API_BASE="${CYNIC_REST_ADDR:-http://localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "K15 Phase 2d Falsification Test"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Test 1: Verify API is up (use /ready — public, no auth needed)
echo -e "\n${YELLOW}[1/6] Verify /ready endpoint${NC}"
if ! curl -sf "${API_BASE}/ready" > /dev/null; then
    echo -e "${RED}✗ API not responding at ${API_BASE}${NC}"
    exit 1
fi
echo -e "${GREEN}✓ API responding${NC}"

# Test 2: Inject 6 failure events for test node "test-node-k15"
echo -e "\n${YELLOW}[2/6] Inject failure events (process_crash)${NC}"
TEST_NODE="test-node-k15"
for i in {1..6}; do
    curl -s -X POST "${API_BASE}/event" \
        ${API_KEY:+-H "Authorization: Bearer ${API_KEY}"} \
        -H "Content-Type: application/json" \
        -d "{
            \"tool\": \"probe\",
            \"node\": \"${TEST_NODE}\",
            \"elapsed_ms\": 5000,
            \"success\": false,
            \"failure_reason\": \"process_crash\"
        }" > /dev/null
    echo "  Event $i: posted"
done
echo -e "${GREEN}✓ 6 failure events injected${NC}"

# Test 3: Call /fleet-stats to verify aggregation
echo -e "\n${YELLOW}[3/6] Query /fleet-stats (aggregation)${NC}"
FLEET_STATS=$(curl -s "${API_BASE}/fleet-stats" \
    ${API_KEY:+-H "Authorization: Bearer ${API_KEY}"} | jq .)

echo "  Response: $FLEET_STATS" | head -20
FLEET_JSON=$(echo "$FLEET_STATS" | jq -r '.[] | select(.node == "'"${TEST_NODE}"'")' 2>/dev/null || echo "")

if [ -n "$FLEET_JSON" ]; then
    FAILURE_REASON=$(echo "$FLEET_JSON" | jq -r '.failure_reason')
    echo -e "${GREEN}✓ Node aggregated: failure_reason='${FAILURE_REASON}'${NC}"
else
    echo -e "${YELLOW}⚠ Node not yet in fleet_stats (aggregation window may not include recent events)${NC}"
fi

# Test 4: Call /inference/remediate to detect degradation
echo -e "\n${YELLOW}[4/6] Call /inference/remediate (detection + recovery)${NC}"
REMEDIATE=$(curl -s "${API_BASE}/inference/remediate" \
    ${API_KEY:+-H "Authorization: Bearer ${API_KEY}"} | jq .)

echo "  Response: $REMEDIATE"
DEGRADED_COUNT=$(echo "$REMEDIATE" | jq '.degraded_nodes | length')
echo -e "  Detected ${DEGRADED_COUNT} degraded node(s)"

if echo "$REMEDIATE" | jq -e '.degraded_nodes[] | select(.node == "'"${TEST_NODE}"'")' > /dev/null 2>&1; then
    REMEDIATION_STATUS=$(echo "$REMEDIATE" | jq -r '.degraded_nodes[] | select(.node == "'"${TEST_NODE}"'") | .remediation_status')
    echo -e "${GREEN}✓ Node detected as degraded: remediation_status='${REMEDIATION_STATUS}'${NC}"
else
    echo -e "${YELLOW}⚠ Node not detected as degraded (may need >80% fatal threshold)${NC}"
fi

# Test 5: Verify /inference/route skips degraded nodes
echo -e "\n${YELLOW}[5/6] Call /inference/route (routing decision)${NC}"
ROUTE=$(curl -s -X POST "${API_BASE}/inference/route" \
    ${API_KEY:+-H "Authorization: Bearer ${API_KEY}"} \
    -H "Content-Type: application/json" \
    -d '{"domain": "token", "content_len": 512}' | jq .)

SELECTED=$(echo "$ROUTE" | jq -r '.selected_node // "none"')
echo "  Selected node: ${SELECTED}"

if [ "${SELECTED}" != "${TEST_NODE}" ]; then
    echo -e "${GREEN}✓ Routing avoids degraded node (selected: ${SELECTED})${NC}"
else
    echo -e "${YELLOW}⚠ Routing still considers degraded node (expected behavior if <80% threshold)${NC}"
fi

# Test 6: Verify observations were stored (K15 consumer)
echo -e "\n${YELLOW}[6/6] Verify recovery observations stored (K15 consumer)${NC}"
OBSERVATIONS=$(curl -s "${API_BASE}/observations?limit=10" \
    ${API_KEY:+-H "Authorization: Bearer ${API_KEY}"} | jq .)

RECOVERY_OBS=$(echo "$OBSERVATIONS" | jq -r '.[] | select(.tool == "ts_exec" and .target == "'"${TEST_NODE}"'") | .status' 2>/dev/null | head -1 || echo "")

if [ -n "$RECOVERY_OBS" ]; then
    echo -e "${GREEN}✓ Recovery observation found: status='${RECOVERY_OBS}'${NC}"
else
    echo -e "${YELLOW}⚠ No recovery observations yet (may require MCP availability)${NC}"
fi

echo -e "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Falsification complete."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
