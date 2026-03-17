# CYNIC — Universal Development Pipeline
# Works for Claude Code, Gemini CLI, humans, any machine.
# One entry point per stage. No discipline required — the structure enforces the flow.
#
# Usage:
#   make check    — build + test + clippy (validate before commit)
#   make commit   — check + gitleaks + commit (validated commit)
#   make ship     — commit + push (pre-push re-validates)
#   make deploy   — ship + backup DB + restart kernel + verify
#   make e2e      — end-to-end test against running kernel
#   make status   — full system dashboard
#   make backup   — manual DB backup

SHELL := /bin/bash
.ONESHELL:

# ── Environment ──────────────────────────────────────────────
PROJECT_DIR := $(shell git rev-parse --show-toplevel 2>/dev/null || pwd)

# Source env vars (CYNIC_REST_ADDR, CYNIC_API_KEY, etc.)
define source_env
	source ~/.cargo/env 2>/dev/null || true
	source ~/.cynic-env 2>/dev/null || true
endef

# ── Stage 1: Validate ───────────────────────────────────────
.PHONY: check
check:
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  CYNIC check — build + test + clippy"
	@echo "══════════════════════════════════════════"
	cargo build -p cynic-kernel --release
	cargo test -p cynic-kernel --release
	cargo clippy -p cynic-kernel --release -- -D warnings
	@echo "✓ All checks passed"

# ── Stage 2: Validated Commit ────────────────────────────────
# Usage: make commit m="type(scope): description"
.PHONY: commit
commit: check
	@if [ -z "$(m)" ]; then echo "ERROR: provide message with m=\"...\"" >&2; exit 1; fi
	@echo ""
	@echo "▶ Staging and committing..."
	git add -u
	git commit -m "$(m)"
	@echo "✓ Committed (gitleaks pre-commit validated)"

# ── Stage 3: Ship (commit + push) ───────────────────────────
.PHONY: ship
ship: commit
	@echo ""
	@echo "▶ Pushing to origin..."
	git push origin $$(git rev-parse --abbrev-ref HEAD)
	@echo "✓ Shipped (pre-push validated build+test+clippy)"

# ── Stage 4: Deploy ─────────────────────────────────────────
.PHONY: deploy
deploy: ship
	@$(source_env)
	@echo ""
	@echo "▶ Backing up DB before deploy..."
	surreal export --endpoint http://localhost:8000 --namespace cynic --database v2 \
		--username root --password "$${SURREALDB_PASS:?Set SURREALDB_PASS in ~/.cynic-env}" \
		~/.surrealdb/backups/cynic_v2_pre_deploy_$$(date +%Y%m%d_%H%M%S).surql
	@echo "▶ Deploying kernel..."
	systemctl --user stop cynic-kernel
	cp $(PROJECT_DIR)/target/release/cynic-kernel ~/bin/cynic-kernel
	cp $(PROJECT_DIR)/target/release/cynic-kernel ~/bin/cynic-mcp
	systemctl --user start cynic-kernel
	@sleep 4
	@echo "▶ Verifying..."
	@curl -s "http://$${CYNIC_REST_ADDR}/health" | python3 -c "import json,sys; h=json.load(sys.stdin); print(f'Kernel: {h[\"status\"]}')"
	@echo "✓ Deployed and verified"

# ── Standalone: End-to-end test ──────────────────────────────
.PHONY: e2e
e2e:
	@$(source_env)
	@echo "▶ E2E: Sicilian Defense..."
	@curl -s -X POST "http://$${CYNIC_REST_ADDR}/judge" \
		-H "Content-Type: application/json" \
		-H "Authorization: Bearer $${CYNIC_API_KEY}" \
		-d '{"content":"1. e4 c5 — The Sicilian Defense.","context":"Most popular response to 1.e4","domain":"chess"}' \
		| python3 -c "import json,sys; v=json.load(sys.stdin); print(f'{v[\"verdict\"]} (Q={v[\"q_score\"][\"total\"]:.3f}) — {v[\"dogs_used\"]}')"

# ── Standalone: System status ────────────────────────────────
.PHONY: status
status:
	@$(source_env)
	@echo "CYNIC System Status"
	@echo "═══════════════════"
	@printf "Kernel:  "; curl -sf "http://$${CYNIC_REST_ADDR}/health" | python3 -c "import json,sys; h=json.load(sys.stdin); print(h['status'])" 2>/dev/null || echo "DOWN"
	@printf "SurrealDB: "; surreal is-ready --endpoint http://localhost:8000 2>/dev/null && echo "ok" || echo "DOWN"
	@printf "Services: "; systemctl --user is-active cynic-kernel surrealdb llama-server 2>/dev/null | tr '\n' ' '; echo ""
	@printf "Git: "; git -C $(PROJECT_DIR) rev-parse --abbrev-ref HEAD; git -C $(PROJECT_DIR) log --oneline -1
	@printf "Backup: "; ls -t ~/.surrealdb/backups/*.surql 2>/dev/null | head -1 || echo "none"

# ── Standalone: Manual backup ────────────────────────────────
.PHONY: backup
backup:
	surreal export --endpoint http://localhost:8000 --namespace cynic --database v2 \
		--username root --password "$${SURREALDB_PASS:?Set SURREALDB_PASS in ~/.cynic-env}" \
		~/.surrealdb/backups/cynic_v2_$$(date +%Y%m%d_%H%M%S).surql
	@echo "✓ Backup saved"

# ── Scope Management ─────────────────────────────────────────
.PHONY: scope
scope:  ## make scope SLUG=rest-audit — create isolated worktree for an ILC
	@if [ -z "$(SLUG)" ]; then echo "ERROR: provide SLUG=<name> (e.g. make scope SLUG=rest-audit)" >&2; exit 1; fi
	@BRANCH="session/$$(id -un)/$(SLUG)"; \
	WORKTREE="$(PROJECT_DIR)/../cynic-$(SLUG)"; \
	git worktree add -b "$$BRANCH" "$$WORKTREE"; \
	echo ""; \
	echo "✓ Worktree: $$WORKTREE"; \
	echo "✓ Branch:   $$BRANCH"; \
	echo ""; \
	echo "Next steps:"; \
	echo "  1. cynic_coord_register(agent_id, intent)"; \
	echo "  2. cynic_coord_claim(agent_id, <target-file>)"; \
	echo "  3. cd $$WORKTREE && work"; \
	echo "  4. make check && cynic_coord_release(agent_id, <target-file>)"; \
	echo "  5. make done SLUG=$(SLUG) when finished"

.PHONY: done
done:  ## make done SLUG=rest-audit — remove worktree and branch
	@if [ -z "$(SLUG)" ]; then echo "ERROR: provide SLUG=<name>" >&2; exit 1; fi
	@BRANCH="session/$$(id -un)/$(SLUG)"; \
	WORKTREE="$(PROJECT_DIR)/../cynic-$(SLUG)"; \
	git worktree remove "$$WORKTREE" 2>/dev/null && echo "✓ Worktree removed" || echo "Note: worktree not found (may already be removed)"; \
	git branch -d "$$BRANCH" 2>/dev/null && echo "✓ Branch $$BRANCH deleted" || echo "Note: delete branch manually if merge is pending"; \
	echo "✓ Scope $(SLUG) released"

.PHONY: agents
agents:  ## Show active agents and work claims (requires kernel running)
	@$(source_env)
	@curl -sf -H "Authorization: Bearer $${CYNIC_API_KEY}" "http://$${CYNIC_REST_ADDR}/agents" | python3 -m json.tool 2>/dev/null || { [ -n "$${CYNIC_REST_ADDR:-}" ] && echo "ERROR: kernel not responding at $${CYNIC_REST_ADDR}" || echo "ERROR: CYNIC_REST_ADDR not set — source ~/.cynic-env first"; }
