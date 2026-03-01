#!/bin/bash
export PYTHONPATH=.

echo "=== TEST RESULTS BY CATEGORY ===" > final_test_results.txt

echo "1. ADAPTERS" | tee -a final_test_results.txt
timeout 120 pytest tests/adapters/ --tb=no -q 2>&1 | tail -3 | tee -a final_test_results.txt

echo -e "\n2. API ROUTERS" | tee -a final_test_results.txt
timeout 120 pytest tests/api/routers/ --tb=no -q 2>&1 | tail -3 | tee -a final_test_results.txt

echo -e "\n3. COGNITION" | tee -a final_test_results.txt
timeout 120 pytest tests/cognition/ --tb=no -q 2>&1 | tail -3 | tee -a final_test_results.txt

echo -e "\n4. CONSENSUS" | tee -a final_test_results.txt
timeout 120 pytest tests/consensus/ --tb=no -q 2>&1 | tail -3 | tee -a final_test_results.txt

echo -e "\n5. INTEGRATIONS" | tee -a final_test_results.txt
timeout 120 pytest tests/integrations/ --tb=no -q 2>&1 | tail -3 | tee -a final_test_results.txt

echo -e "\n6. JUDGES" | tee -a final_test_results.txt
timeout 120 pytest tests/judges/ --tb=no -q 2>&1 | tail -3 | tee -a final_test_results.txt

echo -e "\n7. PROTOCOL" | tee -a final_test_results.txt
timeout 120 pytest tests/protocol/ --tb=no -q 2>&1 | tail -3 | tee -a final_test_results.txt

echo -e "\n8. MCP" | tee -a final_test_results.txt
timeout 120 pytest tests/mcp/ --tb=no -q 2>&1 | tail -3 | tee -a final_test_results.txt

echo -e "\n9. SENSES" | tee -a final_test_results.txt
timeout 120 pytest tests/senses/ --tb=no -q 2>&1 | tail -3 | tee -a final_test_results.txt

echo -e "\n10. CORE ROOT (event bus, state)" | tee -a final_test_results.txt
timeout 120 pytest tests/test_event_bus_integration_clean.py tests/test_unified_state.py --tb=no -q 2>&1 | tail -3 | tee -a final_test_results.txt

echo -e "\n=== SUMMARY ===" | tee -a final_test_results.txt
cat final_test_results.txt

