.PHONY: lifecycle-check
lifecycle-check: ## Resolve lifecycle runtime target without exposing secrets or real IPs
	@$(source_env)
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-check

.PHONY: lifecycle-diagnose
lifecycle-diagnose: ## Print redacted lifecycle resolution even when canonical targets are not ready
	@$(source_env)
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-diagnose

.PHONY: lifecycle-blockers
lifecycle-blockers: ## Print concise lifecycle readiness blockers for UI/machine clients
	@$(source_env)
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers

.PHONY: lifecycle-activation-plan
lifecycle-activation-plan: ## Print registry-derived activation plan for lifecycle operators
	@$(source_env)
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-plan

.PHONY: lifecycle-activation-requests
lifecycle-activation-requests: ## Print organ-generated activation requests from the lifecycle plan
	@$(source_env)
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-requests

.PHONY: lifecycle-activation-dispatch
lifecycle-activation-dispatch: ## Dispatch organ-generated activation requests to the kernel mempool
	@$(source_env)
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-dispatch

.PHONY: lifecycle-runtime-paths
lifecycle-runtime-paths: ## Print registry-derived runtime path requirements for lifecycle nodes
	@$(source_env)
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-runtime-paths

.PHONY: bootstrap-request
bootstrap-request: ## Canonical bootstrap: queue build_runner birth task without requiring lifecycle nodes to be active
	@$(source_env)
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-profile-payloads "bootstrap" "bootstrap-request" "organ-forge-bootstrap" >/dev/null
	@echo "▶ Dispatching bootstrap request via Kernel mempool..."
	@python3 organs/forge/src/entrypoints/cli.py delegate-profile "bootstrap" --json
	@echo "✓ Bootstrap request queued through organ-forge"

.PHONY: deploy-request
deploy-request: ## Canonical: dispatch build on registry build_runner and deploy to kernel_primary
	@$(source_env)
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-profile-payloads "deploy" "deploy-request" "organ-forge-request" >/dev/null
	@$(MAKE) --no-print-directory lifecycle-check
	@echo "▶ Dispatching canonical deploy request via Kernel mempool..."
	@python3 organs/forge/src/entrypoints/cli.py delegate-profile "deploy" --json
	@echo "✓ Deploy request queued through organ-forge"
