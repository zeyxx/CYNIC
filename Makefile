# CYNIC — The Living Organism
# Unified Workflow & Quality Control

.PHONY: help check fix verify tests integrity clean awaken sleep status

help:
	@echo "CYNIC DevOps Command Center"
	@echo "---------------------------"
	@echo "make awaken    : Heartstart infra + Foundation falsification"
	@echo "make sleep     : Stop all infrastructure containers"
	@echo "make status    : Check health of infra & organism"
	@echo "make check     : Run strict linting & type checking (Ruff + Mypy)"
	@echo "make fix       : Auto-fix linting issues"
	@echo "make verify    : Full validation (Lint + Typage + Integrity)"
	@echo "make tests     : Run all fast tests"
	@echo "make integrity : Run agent-side integrity audit"
	@echo "make clean     : Remove caches and temporary files"

check:
	@echo "--- 🔍 RUNNING STRICT LINTING (RUFF) ---"
	ruff check .
	@echo "--- 🧬 RUNNING TYPE CHECKING (MYPY) ---"
	mypy cynic

fix:
	@echo "--- 🛠️  AUTO-FIXING DEBT ---"
	ruff check . --fix
	ruff format .

integrity:
	@echo "--- 🧠 RUNNING INTEGRITY AUDIT ---"
	PYTHONPATH=. python scripts/validate_integrity.py

tests:
	@echo "--- 🧪 RUNNING FAST TESTS ---"
	pytest tests/

verify: check integrity tests
	@echo "--- ✅ ALL SYSTEMS NOMINAL ---"

clean:
	rm -rf .ruff_cache .mypy_cache .pytest_cache
	find . -type d -name "__pycache__" -exec rm -rf {} +

awaken:
	@echo "--- 🧪 AWAKENING CYNIC ---"
	set PYTHONPATH=. && python scripts/heartstart.py
	set PYTHONPATH=. && python scripts/falsify_foundation.py

sleep:
	@echo "--- 💤 PUTTING CYNIC TO SLEEP ---"
	docker-compose stop

status:
	@echo "--- 🏥 INFRASTRUCTURE STATUS ---"
	docker-compose ps
	@echo "--- 🧠 ORGANISM HEALTH ---"
	set PYTHONPATH=. && python scripts/hardware_audit.py
