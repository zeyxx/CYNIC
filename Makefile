# CYNIC — Universal Development Pipeline
# Works for Claude Code, Gemini CLI, humans, any machine.
# One entry point per stage. No discipline required — the structure enforces the flow.
#
# Usage:
#   make check    — build + test + clippy (validate before commit)
#   make commit   — check + gitleaks + commit (validated commit)
#   make ship     — commit + push (pre-push re-validates)
#   make deploy   — ship + backup DB + restart kernel + verify
#   make hotfix   — deploy WITHOUT push (emergency only — skips ship)
#   make rollback — restore previous binary
#   make restore  — restore DB from backup file (F=path)
#   make e2e      — end-to-end test against running kernel
#   make status   — full system dashboard
#   make backup   — manual DB backup
#   make clean    — cargo clean (fix SIGSEGV/LLVM crashes)

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
	@$(MAKE) --no-print-directory lint-rules
	@if surreal is-ready --endpoint http://localhost:8000 2>/dev/null; then \
		echo ""; echo "▶ Integration tests (SurrealDB available)..."; \
		cargo test -p cynic-kernel --release -- --ignored; \
	else \
		echo ""; echo "⚠ SurrealDB not running — skipping integration tests"; \
	fi

.PHONY: lint-rules
lint-rules: ## Grep-enforceable CLAUDE.md rules — mechanical checks the compiler can't do
# Exemptions: probe/ (boot-time hardware discovery), infra/ (health loop, config validation)
# These modules DO raw HTTP but are not backend adapters — they serve infrastructure purposes.
	@echo ""
	@echo "▶ Checking grep-enforceable rules..."
	@FAIL=0; \
	R17=$$(rg 'reqwest' cynic-kernel/src/domain/ cynic-kernel/src/api/ cynic-kernel/src/main.rs 2>/dev/null | grep -v '//'); \
	if [ -n "$$R17" ]; then echo "FAIL Rule #17: reqwest outside backends/storage:"; echo "$$R17"; FAIL=1; fi; \
	R19=$$(rg 'format_crystal_context' cynic-kernel/src/api/ 2>/dev/null | grep -v '//'); \
	if [ -n "$$R19" ]; then echo "FAIL Rule #19: format_crystal_context in handler (should be in pipeline):"; echo "$$R19"; FAIL=1; fi; \
	R32=$$(rg 'serde_json::Value' cynic-kernel/src/domain/ 2>/dev/null | grep -v '//' | grep -v '#\[cfg(test)\]'); \
	if [ -n "$$R32" ]; then echo "FAIL Rule #32: serde_json::Value in domain/:"; echo "$$R32"; FAIL=1; fi; \
	SECRETS=$$(git diff --staged 2>/dev/null | grep -iE 'api.key|token|password|secret|AIza|hf_' | grep -v '#' | grep -v '//'); \
	if [ -n "$$SECRETS" ]; then echo "FAIL Security: possible secrets in staged changes:"; echo "$$SECRETS"; FAIL=1; fi; \
	if [ $$FAIL -eq 0 ]; then echo "✓ All grep-enforceable rules pass"; fi; \
	exit $$FAIL

.PHONY: check-storage
check-storage: ## Integration tests against real SurrealDB (requires :8000)
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  CYNIC check-storage — integration tests"
	@echo "══════════════════════════════════════════"
	cargo test -p cynic-kernel --release -- --ignored
	@echo "✓ All checks passed"

# ── Stage 2: Validated Commit ────────────────────────────────
# Usage: make commit m="type(scope): description"
# Impact checking is done by the compiler (#![deny(dead_code)]) — not bash scripts.
.PHONY: commit
commit: check
	@if [ -z "$(m)" ]; then echo "ERROR: provide message with m=\"...\"" >&2; exit 1; fi
	@echo ""
	@echo "▶ Staging and committing..."
	git add -u
	git commit -m "$(m)"
	@echo "✓ Committed (compiler + clippy + gitleaks validated)"

# ── Stage 3: Ship (commit + push) ───────────────────────────
.PHONY: ship
ship: commit
	@echo ""
	@echo "▶ Pushing to origin..."
	git push origin $$(git rev-parse --abbrev-ref HEAD)
	@echo "✓ Shipped (pre-push validated build+test+clippy)"

# ── Stage 4: Deploy ─────────────────────────────────────────
# Integration tests run BEFORE deploy — if storage queries are broken, don't deploy.
.PHONY: deploy
deploy: ship check-storage
	@$(source_env)
	@echo ""
	@echo "▶ Backing up DB before deploy..."
	surreal export --endpoint http://localhost:8000 --namespace cynic --database v2 \
		--username root --password "$${SURREALDB_PASS:?Set SURREALDB_PASS in ~/.cynic-env}" \
		~/.surrealdb/backups/cynic_v2_pre_deploy_$$(date +%Y%m%d_%H%M%S).surql
	@echo "▶ Deploying kernel (saving previous for rollback)..."
	systemctl --user stop cynic-kernel
	@[ -f ~/bin/cynic-kernel ] && cp ~/bin/cynic-kernel ~/bin/cynic-kernel.prev || true
	cp $(PROJECT_DIR)/target/release/cynic-kernel ~/bin/cynic-kernel
	cp $(PROJECT_DIR)/target/release/cynic-kernel ~/bin/cynic-mcp
	systemctl --user start cynic-kernel
	@echo "▶ Verifying (HTTP status code — no JSON parsing)..."
	@for i in $$(seq 1 15); do \
		sleep 3; \
		HTTP=$$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 \
			$${CYNIC_API_KEY:+-H "Authorization: Bearer $${CYNIC_API_KEY}"} \
			"http://$${CYNIC_REST_ADDR}/health" 2>/dev/null) && \
		[ "$$HTTP" = "200" ] && echo "  Kernel: sovereign (HTTP 200)" && break || \
		[ "$$HTTP" = "503" ] && echo "  Kernel: degraded (HTTP 503) — check Dogs" && break || \
		printf "."; \
	done
	@echo "▶ Healthcheck self-test..."
	@bash ~/bin/cynic-healthcheck.sh && echo "  Healthcheck: PASS" || echo "  ⚠ Healthcheck: FAIL — review ~/bin/cynic-healthcheck.sh"
	@echo "✓ Deployed and verified"

# ── Emergency: Rollback to previous binary ─────────────────
.PHONY: rollback
rollback:
	@$(source_env)
	@[ -f ~/bin/cynic-kernel.prev ] || { echo "✗ No previous binary to rollback to"; exit 1; }
	@echo "▶ Rolling back to previous kernel binary..."
	systemctl --user stop cynic-kernel
	cp ~/bin/cynic-kernel.prev ~/bin/cynic-kernel
	systemctl --user start cynic-kernel
	@sleep 4
	@echo "▶ Verifying rollback..."
	@curl -s "http://$${CYNIC_REST_ADDR}/health" | python3 -c "import json,sys; h=json.load(sys.stdin); print(f'Kernel: {h[\"status\"]} (rolled back)')"

# ── Emergency: Hotfix deploy (skip push — for incidents) ──────
.PHONY: hotfix
hotfix: check
	@$(source_env)
	@echo ""
	@echo "⚠ HOTFIX DEPLOY — skipping push (commit locally, push later)"
	@echo "▶ Backing up DB..."
	surreal export --endpoint http://localhost:8000 --namespace cynic --database v2 \
		--username root --password "$${SURREALDB_PASS:?Set SURREALDB_PASS}" \
		~/.surrealdb/backups/cynic_v2_hotfix_$$(date +%Y%m%d_%H%M%S).surql
	@echo "▶ Deploying kernel..."
	systemctl --user stop cynic-kernel
	@[ -f ~/bin/cynic-kernel ] && cp ~/bin/cynic-kernel ~/bin/cynic-kernel.prev || true
	cp $(PROJECT_DIR)/target/release/cynic-kernel ~/bin/cynic-kernel
	systemctl --user start cynic-kernel
	@echo "▶ Verifying (retry loop)..."
	@for i in $$(seq 1 15); do \
		sleep 3; \
		STATUS=$$(curl -sf "http://$${CYNIC_REST_ADDR}/health" 2>/dev/null | python3 -c "import json,sys;print(json.load(sys.stdin).get('status','?'))" 2>/dev/null) && \
		[ -n "$$STATUS" ] && echo "Kernel: $$STATUS (hotfix)" && break || \
		printf "."; \
	done

# ── Restore DB from backup ────────────────────────────────────
.PHONY: restore
restore:
	@$(source_env)
	@if [ -z "$(F)" ]; then echo "ERROR: provide backup file with F=path"; echo "Available:"; ls -lhrt ~/.surrealdb/backups/ | tail -5; exit 1; fi
	@[ -f "$(F)" ] || { echo "ERROR: file $(F) not found"; exit 1; }
	@echo "⚠ RESTORING DB from $(F) — this will OVERWRITE current data"
	@read -p "Are you sure? (yes/no) " CONFIRM; [ "$$CONFIRM" = "yes" ] || { echo "Aborted"; exit 1; }
	systemctl --user stop cynic-kernel
	surreal import --endpoint http://localhost:8000 --namespace cynic --database v2 \
		--username root --password "$${SURREALDB_PASS}" "$(F)"
	systemctl --user start cynic-kernel
	@echo "✓ DB restored from $(F)"

.PHONY: test-restore
test-restore: ## Verify backup restore works against a test DB (non-destructive)
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  CYNIC test-restore — backup verification"
	@echo "══════════════════════════════════════════"
	@LATEST=$$(ls -t ~/.surrealdb/backups/*.surql.gz ~/.surrealdb/backups/*.surql 2>/dev/null | head -1); \
	if [ -z "$$LATEST" ]; then echo "✗ No backup files found"; exit 1; fi; \
	echo "▸ Testing restore of: $$LATEST"; \
	IMPORT_FILE="$$LATEST"; \
	if echo "$$LATEST" | grep -q '\.gz$$'; then \
		IMPORT_FILE=$$(mktemp /tmp/cynic-restore-test-XXXXXX.surql); \
		gunzip -c "$$LATEST" > "$$IMPORT_FILE"; \
	fi; \
	surreal import --endpoint http://localhost:8000 \
		--namespace cynic_test --database restore_test \
		--username root --password "$${SURREALDB_PASS}" \
		"$$IMPORT_FILE" 2>&1 | tail -3; \
	VCOUNT=$$(echo "USE NS cynic_test; USE DB restore_test; SELECT count() FROM verdict GROUP ALL;" | \
		surreal sql --endpoint http://localhost:8000 --username root --password "$${SURREALDB_PASS}" --hide-welcome 2>/dev/null | grep -oP '\d+' | head -1); \
	CCOUNT=$$(echo "USE NS cynic_test; USE DB restore_test; SELECT count() FROM crystal GROUP ALL;" | \
		surreal sql --endpoint http://localhost:8000 --username root --password "$${SURREALDB_PASS}" --hide-welcome 2>/dev/null | grep -oP '\d+' | head -1); \
	echo "▸ Verdicts: $${VCOUNT:-0}  Crystals: $${CCOUNT:-0}"; \
	echo "USE NS cynic_test; REMOVE DATABASE restore_test;" | \
		surreal sql --endpoint http://localhost:8000 --username root --password "$${SURREALDB_PASS}" --hide-welcome 2>/dev/null; \
	if echo "$$LATEST" | grep -q '\.gz$$'; then rm -f "$$IMPORT_FILE"; fi; \
	if [ "$${VCOUNT:-0}" -gt 0 ]; then echo "✓ Restore verified ($$VCOUNT verdicts, $${CCOUNT:-0} crystals)"; \
	else echo "⚠ Restore completed but 0 verdicts found — check backup contents"; fi

# ── Clean (fix SIGSEGV/LLVM crashes) ──────────────────────────
.PHONY: clean
clean:
	cargo clean -p cynic-kernel
	@echo "✓ Cleaned cynic-kernel build artifacts"

# ── Standalone: End-to-end test ──────────────────────────────
.PHONY: e2e
e2e:
	@$(source_env)
	@echo "▶ E2E: Sicilian Defense..."
	@ADDR="http://$${CYNIC_REST_ADDR}"; AUTH="Authorization: Bearer $${CYNIC_API_KEY}"; FAIL=0; \
	echo ""; \
	printf "  1/6 Health........... "; \
	HTTP=$$(curl -s -o /dev/null -w '%{http_code}' "$$ADDR/health"); \
	[ "$$HTTP" = "200" ] && echo "PASS ($$HTTP)" || { echo "FAIL ($$HTTP)"; FAIL=$$((FAIL+1)); }; \
	printf "  2/6 Judge+dog_scores. "; \
	V=$$(curl -s -X POST "$$ADDR/judge" -H "Content-Type: application/json" -H "$$AUTH" \
		-d '{"content":"1. e4 c5 — The Sicilian Defense.","context":"Most popular response to 1.e4","domain":"chess"}'); \
	DOGS=$$(echo "$$V" | python3 -c "import json,sys; v=json.load(sys.stdin); print(len(v.get('dog_scores',[])))" 2>/dev/null); \
	VERDICT=$$(echo "$$V" | python3 -c "import json,sys; print(json.load(sys.stdin)['verdict'])" 2>/dev/null); \
	[ "$${DOGS:-0}" -gt 0 ] && echo "PASS $$VERDICT dogs=$$DOGS" || { echo "FAIL (dog_scores empty)"; FAIL=$$((FAIL+1)); }; \
	printf "  3/6 Verdict DB RT.... "; \
	DB_DOGS=$$(curl -s -H "$$AUTH" "$$ADDR/verdicts?limit=1" | python3 -c "import json,sys; v=json.load(sys.stdin); print(len(v[0].get('dog_scores',[])) if v else 0)" 2>/dev/null); \
	[ "$${DB_DOGS:-0}" -gt 0 ] && echo "PASS ($$DB_DOGS dogs from DB)" || { echo "FAIL"; FAIL=$$((FAIL+1)); }; \
	printf "  4/6 Observations..... "; \
	OBS=$$(curl -s -H "$$AUTH" "$$ADDR/observations?limit=1" | python3 -c "import json,sys; o=json.load(sys.stdin); print(len(o[0].keys()) if o else 0)" 2>/dev/null); \
	[ "$${OBS:-0}" -ge 10 ] && echo "PASS ($$OBS fields)" || { echo "FAIL ($$OBS fields)"; FAIL=$$((FAIL+1)); }; \
	printf "  5/6 Audit............ "; \
	AUD=$$(curl -s -H "$$AUTH" "$$ADDR/audit?limit=1" | python3 -c "import json,sys; a=json.load(sys.stdin); print(len(a[0].keys()) if a else 0)" 2>/dev/null); \
	[ "$${AUD:-0}" -ge 5 ] && echo "PASS ($$AUD fields)" || { echo "FAIL ($$AUD fields)"; FAIL=$$((FAIL+1)); }; \
	printf "  6/6 Crystals......... "; \
	CRY=$$(curl -s -H "$$AUTH" "$$ADDR/crystals?limit=1" | python3 -c "import json,sys; print(len(json.load(sys.stdin)))" 2>/dev/null); \
	[ "$${CRY:-0}" -gt 0 ] && echo "PASS" || { echo "FAIL"; FAIL=$$((FAIL+1)); }; \
	echo ""; \
	[ $$FAIL -eq 0 ] && echo "✓ All 6 E2E checks passed" || { echo "✗ $$FAIL E2E check(s) FAILED"; exit 1; }

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
