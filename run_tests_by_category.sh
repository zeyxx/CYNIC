#!/bin/bash

PYTHONPATH=. timeout 120 pytest tests/adapters/ --tb=no -q 2>&1 | tee -a test_category_results.txt | tail -5 && echo "---ADAPTERS DONE---" || echo "---ADAPTERS FAILED/TIMEOUT---"
PYTHONPATH=. timeout 120 pytest tests/api/routers/ --tb=no -q 2>&1 | tee -a test_category_results.txt | tail -5 && echo "---API ROUTERS DONE---" || echo "---API ROUTERS FAILED/TIMEOUT---"
PYTHONPATH=. timeout 120 pytest tests/cognition/ --tb=no -q 2>&1 | tee -a test_category_results.txt | tail -5 && echo "---COGNITION DONE---" || echo "---COGNITION FAILED/TIMEOUT---"
PYTHONPATH=. timeout 120 pytest tests/integrations/ --tb=no -q 2>&1 | tee -a test_category_results.txt | tail -5 && echo "---INTEGRATIONS DONE---" || echo "---INTEGRATIONS FAILED/TIMEOUT---"
PYTHONPATH=. timeout 120 pytest tests/protocol/ --tb=no -q 2>&1 | tee -a test_category_results.txt | tail -5 && echo "---PROTOCOL DONE---" || echo "---PROTOCOL FAILED/TIMEOUT---"

