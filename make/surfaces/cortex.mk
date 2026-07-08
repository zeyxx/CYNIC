.PHONY: gate-0
gate-0: ## Level 0 — Instant: fmt + lints + security
	@echo "══════════════════════════════════════════"
	@echo "  Gate Level 0 — Instant checks"
	@echo "══════════════════════════════════════════"
	echo "Skipping cargo fmt (cortex machine)"
	@$(MAKE) --no-print-directory lint-rules
	@$(MAKE) --no-print-directory lint-drift
	@$(MAKE) --no-print-directory lint-volatile-state
	@$(MAKE) --no-print-directory lint-security
	@$(MAKE) --no-print-directory lint-python-tiers
	@$(MAKE) --no-print-directory lint-repo-topology-contract
	@$(MAKE) --no-print-directory lint-agent-exposure-contract
	@$(MAKE) --no-print-directory lint-infra-consumption-contract
	@$(MAKE) --no-print-directory lint-topology
	@$(MAKE) --no-print-directory lint-lifecycle
	@touch .gate-0
	@echo ""
	@echo "  ✓ Gate 0 passed — fmt + lints + security OK"

.PHONY: gate-1
gate-1: ## Level 1 — Light: cargo clippy
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  Gate Level 1 — Clippy"
	@echo "══════════════════════════════════════════"
	cargo clippy --workspace --all-targets -- -D warnings
	@touch .gate-1
	@echo "  ✓ Gate 1 passed — clippy OK"

.PHONY: gate-2
gate-2: ## Level 2 marker check; canonical execution is gate-2-request via organ-forge
	@echo "✗ gate-2 cannot be marked by a cortex-local request."
	@echo "  Use: make gate-2-request"
	@echo "  The .gate-2 marker must be produced by the registry lifecycle executor."
	@exit 1

.PHONY: gate-2-request
gate-2-request: ## Canonical: request gate-2 validation through organ-forge/mempool
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  Gate Level 2 — Registry/MCP Request"
	@echo "══════════════════════════════════════════"
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-profile-payloads "gate-2" "gate-2-request" "organ-forge-request" >/dev/null
	@$(MAKE) --no-print-directory lifecycle-check
	@echo "▶ Requesting validation profile through organ-forge..."
	@python3 organs/forge/src/entrypoints/cli.py delegate-profile "gate-2" --json
	@echo "  ✓ Gate 2 request queued; no local .gate-2 marker written"

.PHONY: mcp-reload-request
mcp-reload-request: ## Canonical: request cynic-mcp build/reload through organ-forge/mempool
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  MCP Reload — Registry/MCP Request"
	@echo "══════════════════════════════════════════"
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-profile-payloads "mcp-reload" "mcp-runtime-request" "organ-forge-request" >/dev/null
	@$(MAKE) --no-print-directory lifecycle-check
	@echo "▶ Requesting MCP reload profile through organ-forge..."
	@python3 organs/forge/src/entrypoints/cli.py delegate-profile "mcp-reload" --json
	@echo "  ✓ MCP reload request queued; no cortex-local build executed"

.PHONY: mcp-repair-local
mcp-repair-local: ## Emergency: locally rebuild cynic-mcp when MCP/kernel/build-runner path is broken
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  MCP Repair — Cortex-local emergency build"
	@echo "══════════════════════════════════════════"
	@echo "This is a repair build only. It is not gate-2, release validation, or deploy."
	if [[ "$${profile:-dev}" == "release" ]]; then
		cargo build -p cynic-mcp --release
		chmod +x target/release/cynic-mcp
		export CYNIC_MCP_BINARY="$(PROJECT_DIR)/target/release/cynic-mcp"
	else
		cargo build -p cynic-mcp
		chmod +x target/debug/cynic-mcp
		export CYNIC_MCP_BINARY="$(PROJECT_DIR)/target/debug/cynic-mcp"
	fi
	export CYNIC_PROJECT_ROOT="$(PROJECT_DIR)"
	export CYNIC_MCP_REPAIR_LOCAL=1
	@$(MAKE) --no-print-directory mcp-freshness
	@echo "  ✓ Local MCP repair build completed"

.PHONY: check
check: gate-0 gate-1
	@echo ""
	@echo "  ✓ Local cortex gates passed"
	@echo "  Release validation: make gate-2-request"

.PHONY: gate
gate: check
	@touch .gate-passed
	@echo ""
	@echo "══════════════════════════════════════════"
	@echo "  ✓ Local gates passed — marker written"
	@echo "  Now run: git push"
	@echo "══════════════════════════════════════════"
