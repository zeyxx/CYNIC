# CYNIC Python Kernel — Makefile
# "φ distrusts φ" — κυνικός
#
# Usage:
#   make up           — Start all services (postgres + ollama + kernel)
#   make down         — Stop all services
#   make restart      — Restart cynic-kernel only (fast iteration)
#   make logs         — Follow all logs
#   make logs-kernel  — Follow kernel logs only
#   make pull-models  — Pull required Ollama models
#   make test         — Run unit tests (no DB/Ollama needed)
#   make test-all     — Run ALL tests including integration
#   make shell        — Open shell in cynic-kernel container
#   make psql         — Open psql in postgres-py container
#   make migrate      — Re-run DB migration (create_tables)
#   make clean        — Remove containers + volumes (DATA LOSS)

COMPOSE := docker compose -f docker-compose.dev.yml
KERNEL_SVC := cynic-kernel
PG_SVC := postgres-py
OLLAMA_SVC := ollama

# Default Ollama models (scoring + embeddings)
OLLAMA_MODELS := gemma2:2b nomic-embed-text

.PHONY: up down restart logs logs-kernel pull-models \
        test test-all shell psql migrate clean \
        build status

## ── Lifecycle ────────────────────────────────────────────────────────────

up:
	$(COMPOSE) up -d --build
	@echo ""
	@echo "  CYNIC kernel: http://localhost:8000"
	@echo "  Ollama:       http://localhost:11434"
	@echo "  PostgreSQL:   localhost:5433 (DB: cynic_py)"
	@echo ""
	@echo "  Run 'make pull-models' to download LLM models."

down:
	$(COMPOSE) down

restart:
	$(COMPOSE) restart $(KERNEL_SVC)

build:
	$(COMPOSE) build $(KERNEL_SVC)

status:
	$(COMPOSE) ps

## ── Logs ─────────────────────────────────────────────────────────────────

logs:
	$(COMPOSE) logs -f

logs-kernel:
	$(COMPOSE) logs -f $(KERNEL_SVC)

logs-db:
	$(COMPOSE) logs -f $(PG_SVC)

## ── Ollama Models ────────────────────────────────────────────────────────

pull-models:
	@echo "Pulling Ollama models: $(OLLAMA_MODELS)"
	@for model in $(OLLAMA_MODELS); do \
		echo "  Pulling $$model..."; \
		$(COMPOSE) exec $(OLLAMA_SVC) ollama pull $$model; \
	done
	@echo "Models ready."

list-models:
	$(COMPOSE) exec $(OLLAMA_SVC) ollama list

## ── Testing ──────────────────────────────────────────────────────────────

test:
	python -m pytest tests/ \
		--ignore=tests/test_db_integration.py \
		--ignore=tests/test_integration.py \
		--ignore=tests/test_api_integration.py \
		-q

test-all:
	python -m pytest tests/ -q

test-integration:
	CYNIC_DATABASE_URL=postgresql://cynic:cynic_dev@localhost:5433/cynic_py \
	CYNIC_OLLAMA_URL=http://localhost:11434 \
	python -m pytest tests/test_db_integration.py tests/test_api_integration.py \
		tests/test_integration.py -v

test-watch:
	python -m pytest tests/ \
		--ignore=tests/test_db_integration.py \
		--ignore=tests/test_integration.py \
		--ignore=tests/test_api_integration.py \
		-q --tb=short -f

## ── Database ─────────────────────────────────────────────────────────────

psql:
	$(COMPOSE) exec $(PG_SVC) psql -U cynic -d cynic_py

migrate:
	@echo "Running create_tables() via kernel..."
	$(COMPOSE) exec $(KERNEL_SVC) python -c \
		"import asyncio; from cynic.core.storage.postgres import create_tables; asyncio.run(create_tables())"

db-reset:
	@echo "WARNING: This will drop and recreate the database!"
	@read -p "Are you sure? [y/N] " confirm; \
	if [ "$$confirm" = "y" ]; then \
		$(COMPOSE) exec $(PG_SVC) psql -U cynic -c "DROP DATABASE IF EXISTS cynic_py;" postgres; \
		$(COMPOSE) exec $(PG_SVC) psql -U cynic -c "CREATE DATABASE cynic_py;" postgres; \
		$(COMPOSE) exec $(PG_SVC) psql -U cynic -d cynic_py -c "CREATE EXTENSION IF NOT EXISTS vector;"; \
		$(MAKE) migrate; \
	fi

## ── Kernel Shell ─────────────────────────────────────────────────────────

shell:
	$(COMPOSE) exec $(KERNEL_SVC) /bin/bash

## ── Cleanup ──────────────────────────────────────────────────────────────

clean:
	@echo "WARNING: This removes containers AND volumes (all data lost)!"
	@read -p "Are you sure? [y/N] " confirm; \
	if [ "$$confirm" = "y" ]; then \
		$(COMPOSE) down -v --remove-orphans; \
	fi
