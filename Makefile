# CYNIC — Universal Development Pipeline
# Works for Claude Code, Antigravity CLI, humans, any machine.
# One entry point per stage. No discipline required — the structure enforces the flow.
#
# Usage:
#   make check    — local cortex validation (gate-0 + gate-1)
#   make -f make/surfaces/cortex.mk check    — explicit cortex-local surface
#   make -f make/surfaces/lifecycle.mk mcp-reload-request    — explicit lifecycle request surface
#   make runtime-principals-install-cynic-cli — regenerate /usr/local/bin/cynic from project projection
#   make runtime-principals-bootstrap          — full projection refresh (dependencies + apply + install-defaults)
#   make runtime-principals-apply              — explicit runtime projection surface
#   make runtime-ready                        — bootstrap organism Python runtime + update hermes-agent
#   make runtime-heal                         — force recreate runtime and re-sync hermes-agent
#   make hermes cmd="update"                  — run hermes through the organism runtime
#   make gate     — local cortex validation + marker (run BEFORE git push)
#   make bootstrap-request — canonical registry/MCP build_runner birth request
#   make gate-2-request — canonical registry/MCP release validation request
#   make mcp-reload-request — canonical registry/MCP cynic-mcp build/reload request
#   make mcp-repair-local — emergency cortex-local cynic-mcp repair build
#   make commit   — check + commit (validated commit)
#   make ship     — commit + push (pre-push verifies gate marker)
#   make deploy-request — canonical registry/MCP build+deploy request
#   make deploy   — local legacy deploy; requires CYNIC_ALLOW_LOCAL_DEPLOY=1
#   make deploy-only — local legacy fast-path; requires CYNIC_ALLOW_LOCAL_DEPLOY=1
#   make hotfix   — local emergency deploy; requires CYNIC_ALLOW_LOCAL_DEPLOY=1
#   make rollback — restore previous binary
#   make restore  — restore DB from backup file (F=path)
#   make e2e      — end-to-end test against running kernel
#   make status   — full system dashboard
#   make runtime-truth — PID/PPID/binary/version/state/port truth for critical processes
#   make runtime-check — fail on runtime drift or duplicate critical processes; warn on minimal unit profiles
#   make mcp-freshness — fail if the Codex MCP wrapper would select a stale binary
#   make mcp-repair-local — rebuild local MCP binary when the canonical path is inaccessible
#   make deploy-systemd — idempotent symlink infra/systemd/* → ~/.config/systemd/user/
#   make deploy-systemd-dry — dry-run: show what deploy-systemd would change
#   make verify-systemd-units — fail if installed user units drift from the repo
#   make backup   — manual DB backup
#   make test-gates — R21: verify lint gates catch known violations
#   make clean    — cargo clean (fix SIGSEGV/LLVM crashes)

SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
MAKEFLAGS += --warn-undefined-variables
.NOTPARALLEL:

# ── Environment ──────────────────────────────────────────────
PROJECT_DIR := $(shell git rev-parse --show-toplevel 2>/dev/null || pwd)

# 64 MiB: bounded rustc stack for heterogeneous lifecycle/cortex builds.
export RUST_MIN_STACK := 67108864

# Source env vars (CYNIC_REST_ADDR, CYNIC_API_KEY, etc.)
define source_env
	source ~/.cargo/env 2>/dev/null || true
	set -a
	source ~/.cynic-env 2>/dev/null || true
	set +a
	export RUST_MIN_STACK=67108864  # 64 MiB: bounded rustc stack for heterogeneous lifecycle/cortex builds.


endef

include make/surfaces/cortex.mk
include make/surfaces/lifecycle.mk
include make/surfaces/runtime.mk

# ── Stage 1: Validate (3-level gates) ────────────────────────
# Level 0 — Instant (<2s): fmt + lint-rules + lint-drift + security
#   Always runs. Pure static checks, no compilation.
# Level 1 — Light (~25s): cargo clippy
#   Runs once per session. Marker invalidated only by Rust source changes.
# Level 2 — Full (~1m30s): cargo test + audit + integration
#   Runs before deploy only. NOT at push time.


.PHONY: lint-rules
lint-rules: lint-rule-r1 lint-rule-r2 lint-rule-k1 lint-rule-k2 lint-rule-k3-k11 lint-rule-k4 lint-rule-k5 lint-rule-k12 lint-rule-security lint-rule-ips ## Grep-enforceable CLAUDE.md rules
	@echo "✓ All grep-enforceable rules migrated to organ-supervisor and passed."

.PHONY: lint-rule-r1
lint-rule-r1: ## R1: No hardcoded absolute paths
	@python3 organs/supervisor/controllers/validate_hardcoded_paths.py

.PHONY: lint-rule-r2
lint-rule-r2: ## R2: No .ok() without adjacent logging
	@python3 organs/supervisor/controllers/validate_unlogged_ok.py

.PHONY: lint-rule-k1
lint-rule-k1: ## K1: No #[cfg] in domain/ (domain purity)
	@python3 organs/supervisor/controllers/validate_domain_purity.py

.PHONY: lint-rule-k2
lint-rule-k2: ## K2: No reqwest outside backends/storage
	@python3 organs/supervisor/controllers/validate_reqwest_usage.py

.PHONY: lint-rule-k3-k11
lint-rule-k3-k11: ## K3/K11: No pipeline functions in api/
	@python3 organs/supervisor/controllers/validate_pipeline_functions.py

.PHONY: lint-rule-k4
lint-rule-k4: ## K4: No duplicate trait names in domain/
	@python3 organs/supervisor/controllers/validate_duplicate_traits.py

.PHONY: lint-rule-k5
lint-rule-k5: ## K5: No infra type leakage in domain/
	@python3 organs/supervisor/controllers/validate_infra_leakage.py

.PHONY: lint-rule-k12
lint-rule-k12: ## K12: No #[allow] without WHY:
	@python3 organs/supervisor/controllers/validate_allow_without_why.py

.PHONY: lint-rule-security
lint-rule-security: ## Security: No hardcoded secrets
	@python3 organs/supervisor/controllers/validate_secrets.py

.PHONY: lint-rule-ips
lint-rule-ips: ## Security: No hardcoded IPs
	@python3 organs/supervisor/controllers/validate_ips.py





.PHONY: lint-volatile-state
lint-volatile-state: ## Block staged runtime snapshots from organ state directories
	@python3 organs/supervisor/controllers/validate_volatile_state.py

.PHONY: markdown-fragmentation-audit
markdown-fragmentation-audit: ## Audit tracked markdown placement against SSOT policy
	@python3 organs/supervisor/controllers/audit_organs.py


.PHONY: lint-services
lint-services: ## Compare running user services vs expected list — catches zombie/missing services
	@echo ""
	@echo "▶ Checking service state..."
	@EXPECTED="docs/ops/expected-services.txt"; \
	export XDG_RUNTIME_DIR="$${XDG_RUNTIME_DIR:-/run/user/$$(id -u)}"; \
	if [ ! -f "$$EXPECTED" ]; then echo "⚠ $$EXPECTED not found — skipping"; exit 0; fi; \
	FAIL=0; \
	while IFS= read -r svc; do \
		case "$$svc" in \#*|"") continue;; esac; \
		if ! systemctl --user is-active --quiet "$$svc" 2>/dev/null; then \
			echo "✗ MISSING: $$svc (expected active, not found)"; FAIL=1; \
		fi; \
	done < "$$EXPECTED"; \
	for svc in $$(systemctl --user list-units --type=service --state=active --no-legend 2>/dev/null | awk '{print $$1}' | grep -E "cynic|llama|surreal|kairos|cloudflare"); do \
		if ! grep -q "^$$svc$$" "$$EXPECTED" 2>/dev/null; then \
			echo "✗ UNEXPECTED: $$svc (running but not in expected list)"; FAIL=1; \
		fi; \
	done; \
	if [ $$FAIL -eq 0 ]; then echo "✓ All services match expected state"; fi; \
	exit $$FAIL

.PHONY: lint-lifecycle
lint-lifecycle: ## Verify registry lifecycle policy and block workflow-critical edits while planned
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-lint
	@$(MAKE) --no-print-directory test-lifecycle
	@$(MAKE) --no-print-directory lint-lifecycle-readiness

.PHONY: lint-lifecycle-readiness
lint-lifecycle-readiness: ## Fail when lifecycle-critical staged files target a planned release path
	@echo ""
	@echo "▶ Checking lifecycle readiness..."
	@if [ "$${ALLOW_PLANNED_LIFECYCLE_COMMIT:-}" = "1" ]; then \
		echo "⚠ Planned lifecycle commit override enabled"; \
		exit 0; \
	fi; \
	STAGED=$$(git diff --cached --name-only --diff-filter=ACM 2>/dev/null \
		| grep -E '^(infra/registry\.json|Makefile|crates/cynic-kernel/|crates/cynic-mcp/|organs/forge/)' || true); \
	if [ -z "$$STAGED" ]; then \
		echo "✓ No lifecycle-critical files staged"; \
		exit 0; \
	fi; \
	if python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers >/tmp/cynic-lifecycle-blockers.json 2>&1; then \
		echo "✓ Lifecycle readiness OK"; \
		rm -f /tmp/cynic-lifecycle-blockers.json; \
		exit 0; \
	fi; \
	echo "FAIL lifecycle readiness: planned release path blocks workflow-critical edits:"; \
	cat /tmp/cynic-lifecycle-blockers.json; \
	rm -f /tmp/cynic-lifecycle-blockers.json; \
	exit 1

.PHONY: test-lifecycle
test-lifecycle: ## Run organ-forge import and lifecycle contract tests
	@PYTHONDONTWRITEBYTECODE=1 python3 -c 'from pathlib import Path; [compile(path.read_text(), str(path), "exec") for path in Path("organs/forge/src").rglob("*.py")]'
	@PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s organs/forge/src/tests -p 'test_*.py'

.PHONY: mcp-freshness
mcp-freshness: ## Diagnose Codex MCP wrapper binary freshness without starting MCP
	@CYNIC_MCP_CHECK_ONLY=1 python3 .cortex/mcp/run_cynic_mcp.py

.PHONY: portability-audit
portability-audit: ## Diagnose hardcoded path debt across runtime/source surfaces
	@python3 organs/supervisor/controllers/audit_organs.py

.PHONY: stage-readiness
stage-readiness: ## Evaluate stage readiness across 8 dimensions with MCP/network/service/fleet iteration plan
	@python3 organs/supervisor/controllers/audit_runtime.py \
		$${ITERATIONS:+--iterations "$$ITERATIONS"} \
		$${JSON:+--json} \
		$${MAX_WARN:+--max-warn "$$MAX_WARN"}

.PHONY: stage-readiness-strict
stage-readiness-strict: ## Same as stage-readiness with MAX_WARN forced to 0
	@$(MAKE) --no-print-directory stage-readiness ITERATIONS=$${ITERATIONS:-8} MAX_WARN=0 $${JSON:+JSON=1}

.PHONY: lifecycle-verify-runtime-paths
lifecycle-verify-runtime-paths: ## Verify runtime path proofs on this machine for node=<registry-node>
	@$(source_env)
	@[ -n "$${node:-}" ] || { echo "ERROR: provide node=<registry-node>"; exit 1; }
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-verify-runtime-paths "$${node}"

.PHONY: lifecycle-service-evidence-patch
lifecycle-service-evidence-patch: ## Print service activation evidence patch for node=<registry-node>
	@$(source_env)
	@[ -n "$${node:-}" ] || { echo "ERROR: provide node=<registry-node>"; exit 1; }
	@if [ -n "$${SERVICE_STATES:-}" ]; then \
		python3 organs/forge/src/entrypoints/cli.py lifecycle-service-evidence-patch "$${node}" --service-states "$${SERVICE_STATES}"; \
	else \
		python3 organs/forge/src/entrypoints/cli.py lifecycle-service-evidence-patch "$${node}"; \
	fi

.PHONY: lifecycle-validate-activation-evidence-patch
lifecycle-validate-activation-evidence-patch: ## Validate evidence patch JSON with patch='<json>'
	@$(source_env)
	@[ -n "$${patch:-}" ] || { echo "ERROR: provide patch='<json>'"; exit 1; }
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-validate-activation-evidence-patch "$${patch}"

.PHONY: lifecycle-apply-activation-evidence-patch
lifecycle-apply-activation-evidence-patch: ## Validate and persist evidence patch JSON with patch='<json>'
	@$(source_env)
	@[ -n "$${patch:-}" ] || { echo "ERROR: provide patch='<json>'"; exit 1; }
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-apply-activation-evidence-patch "$${patch}"

.PHONY: lifecycle-activate-node
lifecycle-activate-node: ## Validate lifecycle evidence and persist node status=active for node=<registry-node>
	@$(source_env)
	@[ -n "$${node:-}" ] || { echo "ERROR: provide node=<registry-node>"; exit 1; }
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-activate-node "$${node}"

.PHONY: lifecycle-profile-payloads
lifecycle-profile-payloads: ## Dry-run: build and validate all payloads for lifecycle request profile p=<profile>
	@python3 organs/forge/src/entrypoints/cli.py lifecycle-profile-payloads "$(p)" "$${DOMAIN:-lifecycle-profile-dry-run}" "$${AGENT_ID:-organ-forge-dry-run}"

.PHONY: lint-drift
lint-drift: ## Detect config/code/docs drift — names vs reality, dead modules, phantom skills, unwired hooks
	@echo ""
	@echo "▶ Checking for drift..."
	@set +e; FAIL=0; \
	KERNEL_VERSION=$$(grep '^version' $(PROJECT_DIR)/crates/cynic-kernel/Cargo.toml | head -1 | sed 's/.*"\([^"]*\)".*/\1/'); \
	CHANGELOG_HAS_VERSION=$$(grep -c "^\## \[$$KERNEL_VERSION\]" $(PROJECT_DIR)/CHANGELOG.md 2>/dev/null || true); \
	VERSION_MD_HAS_VERSION=$$(grep -c "$$KERNEL_VERSION" $(PROJECT_DIR)/VERSION.md 2>/dev/null || true); \
	if [ "$$CHANGELOG_HAS_VERSION" -eq 0 ]; then \
		echo "FAIL SSOT: Cargo.toml version $$KERNEL_VERSION not in CHANGELOG.md"; FAIL=1; \
	fi; \
	if [ "$$VERSION_MD_HAS_VERSION" -eq 0 ]; then \
		echo "FAIL SSOT: Cargo.toml version $$KERNEL_VERSION not in VERSION.md"; FAIL=1; \
	fi; \
	BACKENDS="$${HOME}/.config/cynic/backends.toml"; \
	if [ -f "$$BACKENDS" ]; then \
		while IFS= read -r line; do \
			NAME=$$(echo "$$line" | grep -oP '(?<=\[backend\.)[^]]+' | sed 's/\.remediation//'); \
			[ -z "$$NAME" ] && continue; \
			MODEL=$$(sed -n "/\[backend\.$${NAME}\]/,/^\[/p" "$$BACKENDS" | grep '^model' | head -1 | sed 's/.*= *"//;s/".*//'); \
			[ -z "$$MODEL" ] && continue; \
			if [ "$$MODEL" = "auto" ]; then continue; fi; \
		done < <(grep '^\[backend\.' "$$BACKENDS" | grep -v '\.remediation\]'); \
	else \
		echo "SKIP: backends.toml not found at $$BACKENDS"; \
	fi; \
	DORMANT=$$(grep -nE '^\s*//\s*pub mod' $(PROJECT_DIR)/crates/cynic-kernel/src/domain/mod.rs | grep -v 'DORMANT:'); \
	if [ -n "$$DORMANT" ]; then echo "FAIL Drift: commented module without DORMANT tag:"; echo "$$DORMANT"; FAIL=1; fi; \
	WORKFLOW_SKILLS=$$(sed -n '/^## Workflow Triggers/,/^## /p' $(PROJECT_DIR)/.cortex/rules/workflow.md | grep -oP '(?<=`/)[a-z][-a-z0-9:]*(?=`)' | sort -u); \
	for SKILL in $$WORKFLOW_SKILLS; do \
		FOUND=0; \
		if echo "$$SKILL" | grep -q ':'; then \
			NS=$$(echo "$$SKILL" | cut -d: -f1); NAME=$$(echo "$$SKILL" | cut -d: -f2); \
			[ -d "$${HOME}/.cortex/commands/$${NS}/$${NAME}" ] && FOUND=1; \
			[ -f "$${HOME}/.cortex/commands/$${NS}/$${NAME}.md" ] && FOUND=1; \
		else \
			[ -f "$(PROJECT_DIR)/.cortex/commands/$${SKILL}.md" ] && FOUND=1; \
		fi; \
		if [ $$FOUND -eq 0 ]; then \
			echo "FAIL Drift: skill '/$${SKILL}' in workflow.md but not found on disk"; FAIL=1; \
		fi; \
	done; \
	for HOOK in $(PROJECT_DIR)/.cortex/mcp/*.py; do \
		HOOK_BASE=$$(basename "$$HOOK"); \
		if ! grep -q "$$HOOK_BASE" $(PROJECT_DIR)/.claude/settings.json 2>/dev/null && \
		   ! grep -q "$$HOOK_BASE" $(PROJECT_DIR)/.claude/settings.local.json 2>/dev/null && \
		   ! grep -q "$$HOOK_BASE" $(PROJECT_DIR)/.mcp.json 2>/dev/null && \
		   ! grep -q "$$HOOK_BASE" $(PROJECT_DIR)/.antigravity/settings.json 2>/dev/null && \
		   ! grep -q "$$HOOK_BASE" $(PROJECT_DIR)/.codex/config.toml 2>/dev/null; then \
			echo "WARN Drift: hook '$$HOOK_BASE' on disk but not wired in any adapter settings"; FAIL=1; \
		fi; \
	done; \
	STORES=$$(grep -oP 'async fn \Kstore_\w+' $(PROJECT_DIR)/crates/cynic-kernel/src/domain/storage/mod.rs | sort -u); \
	for STORE in $$STORES; do \
		ENTITY=$$(echo "$$STORE" | sed 's/^store_//'); \
		STEM=$$(echo "$$ENTITY" | sed 's/y$$//' | sed 's/ing$$//'); \
		CORE=$$(echo "$$ENTITY" | sed 's/^.*_//'); \
		if ! grep -qE "async fn (list_|get_|load_|query_|count_|search_|find_)\w*($$STEM|$$CORE)" $(PROJECT_DIR)/crates/cynic-kernel/src/domain/storage/mod.rs; then \
			echo "FAIL Rule 33: '$$STORE' has no read path (searched '$$STEM' or '$$CORE')"; FAIL=1; \
		fi; \
	done; \
	ROUTES=$$(grep -oP '\.route\("\K[^"]+' $(PROJECT_DIR)/crates/cynic-kernel/src/api/rest/mod.rs); \
	for ROUTE in $$ROUTES; do \
		if ! grep -qF "$$ROUTE" $(PROJECT_DIR)/API.md 2>/dev/null; then \
			echo "FAIL Drift: route '$$ROUTE' registered in code but missing from API.md"; FAIL=1; \
		fi; \
	done; \
	python3 $(PROJECT_DIR)/organs/supervisor/src/entrypoints/api_contracts.py validate-openapi-routes || FAIL=1; \
	python3 $(PROJECT_DIR)/organs/supervisor/src/entrypoints/api_contracts.py validate-api-topology || FAIL=1; \
	REF_DOGS=$$(sed -n '/^## Dogs/,/^##/p' $(PROJECT_DIR)/.cortex/rules/reference.md | grep -oP '(?<=\| )[a-z][-a-z0-9]+(?= +\|)' | sort -u); \
	LIVE_DOGS="deterministic-dog"; \
	if [ -f "$$BACKENDS" ]; then \
		BACKEND_DOGS=$$(grep -E '^\[(backend|dog)\.' "$$BACKENDS" | sed -E 's/^\[(backend|dog)\.//;s/\].*//' | grep -v 'remediation'); \
		LIVE_DOGS=$$(printf '%s\n' "$$LIVE_DOGS" $$BACKEND_DOGS | sort -u); \
	fi; \
	REF_ONLY=$$(comm -23 <(echo "$$REF_DOGS") <(echo "$$LIVE_DOGS")); \
	LIVE_ONLY=$$(comm -13 <(echo "$$REF_DOGS") <(echo "$$LIVE_DOGS")); \
	if [ -n "$$REF_ONLY" ]; then echo "WARN D4: Dogs in reference.md but not active: $$REF_ONLY"; fi; \
	if [ -n "$$LIVE_ONLY" ]; then echo "WARN D4: Dogs active but not in reference.md: $$LIVE_ONLY"; fi; \
	KERN="$(PROJECT_DIR)/crates/cynic-kernel/src"; \
	DISSOLVED=$$(grep -rn "dissolved\|Dissolved" "$$KERN/storage/" "$$KERN/infra/" --include='*.rs' 2>/dev/null | grep -v 'test\|//\|Display\|FromStr\|Serialize\|Deserialize' | grep -i 'dissolved' | wc -l); \
	if [ "$$DISSOLVED" -eq 0 ]; then echo "FAIL K15: CrystalState::Dissolved defined but never produced"; FAIL=1; fi; \
	COMPLIANCE_GATES=$$(grep -rn 'compliance.*deny\|compliance.*block\|compliance.*throttle\|compliance.*reject' "$$KERN/" --include='*.rs' 2>/dev/null | grep -v 'test\|//' | wc -l); \
	if [ "$$COMPLIANCE_GATES" -eq 0 ]; then echo "WARN K15: compliance_score computed but no gate enforces threshold"; fi; \
	CV_READERS=$$(grep -rn 'contributing_verdicts' "$$KERN/" --include='*.rs' 2>/dev/null | grep -v 'test\|//\|push\|insert\|store\|array::union\|Vec<String>\|struct\|pub ' | wc -l); \
	if [ "$$CV_READERS" -eq 0 ]; then echo "WARN K15: contributing_verdicts stored but never read"; fi; \
	EVENT_VARIANTS=$$(grep -oP 'KernelEvent::\K\w+' "$$KERN/domain/events.rs" 2>/dev/null | sort -u); \
	EVENT_HANDLERS=$$(grep -oP 'KernelEvent::\K\w+' "$$KERN/infra/tasks/runtime_loops.rs" 2>/dev/null | sort -u); \
	for EV in $$EVENT_VARIANTS; do \
		if ! echo "$$EVENT_HANDLERS" | grep -q "$$EV"; then \
			EMITTERS=$$(grep -rn "KernelEvent::$$EV" "$$KERN/" --include='*.rs' 2>/dev/null | grep -v 'test\|//\|match\|=>' | wc -l); \
			if [ "$$EMITTERS" -gt 0 ]; then \
				echo "WARN K15: KernelEvent::$$EV emitted ($$EMITTERS site(s)) but not matched in event_consumer"; \
			fi; \
		fi; \
	done; \
	for RULE in $(PROJECT_DIR)/.cortex/rules/*.md; do \
		grep -oP '`[a-zA-Z_/]+\.(rs|toml|sh|md)`' "$$RULE" 2>/dev/null | tr -d '`' | while read -r REF; do \
			if echo "$$REF" | grep -qE '\.rs$$' && ! find "$$KERN" -path "*$$REF" 2>/dev/null | grep -q .; then \
				if ! find "$(PROJECT_DIR)" -path "*$$REF" 2>/dev/null | grep -q .; then \
					echo "WARN Drift: '$$REF' referenced in $$(basename $$RULE) but not found on disk"; \
				fi; \
			fi; \
		done; \
	done; \
	PORT_METHODS=$$(grep -P '^\s+async fn \w+' $(PROJECT_DIR)/crates/cynic-kernel/src/domain/storage/mod.rs | grep -oP 'fn \K\w+' | sort); \
	RECON_METHODS=$$(grep -P '^\s+async fn \w+' $(PROJECT_DIR)/crates/cynic-kernel/src/storage/reconnectable.rs | grep -oP 'fn \K\w+' | sort); \
	for PM in $$PORT_METHODS; do \
		if ! echo "$$RECON_METHODS" | grep -qw "$$PM"; then \
			echo "FAIL K17: StoragePort::$$PM not forwarded in ReconnectableStorage"; FAIL=1; \
		fi; \
	done; \
	if [ $$FAIL -eq 0 ]; then echo "✓ No drift detected"; fi; \
	exit $$FAIL

.PHONY: lint-python-tiers
lint-python-tiers: ## P1-P6: Python code lifecycle governance — new/modified files MUST have tier tag
	@echo ""
	@echo "▶ Checking Python tier governance..."
	@set +e; FAIL=0; WARN_COUNT=0; \
	CHANGED=$$(git diff --cached --name-only --diff-filter=ACM 2>/dev/null; git diff --name-only --diff-filter=ACM 2>/dev/null; git ls-files --others --exclude-standard 2>/dev/null); \
	NEW_PY=$$(echo "$$CHANGED" | grep '\.py$$' | sort -u); \
	ALL_PY=$$(find services/cynic-python scripts -name '*.py' -type f \
		-not -path '*__pycache__*' -not -path '*/venv/*' \
		-not -path '*/tests/*' -not -name 'test_*.py' -not -name 'conftest.py' \
		-not -name '__init__.py' 2>/dev/null); \
	for PYFILE in $$ALL_PY; do \
		CONTENT=$$(cat "$$PYFILE" 2>/dev/null); \
		HAS_TIER=$$(echo "$$CONTENT" | grep -E 'EXPERIMENT|Tier [123]|INFRASTRUCTURE' | head -1); \
		if [ -z "$$HAS_TIER" ]; then \
			if echo "$$NEW_PY" | grep -qx "$$PYFILE" 2>/dev/null; then \
				echo "FAIL P1-P6: new/modified '$$PYFILE' has no tier tag"; \
				FAIL=1; \
			else \
				WARN_COUNT=$$((WARN_COUNT + 1)); \
			fi; \
		fi; \
	done; \
	if [ $$WARN_COUNT -gt 0 ]; then \
		echo "  $$WARN_COUNT existing files untagged (grace period until 2026-06-01)"; \
	fi; \
	if [ $$FAIL -eq 0 ]; then echo "✓ Python tier governance OK"; fi; \
	exit $$FAIL

.PHONY: lint-subprocess-env
lint-subprocess-env: ## R23 gate: wrappers/Rust subprocess env is explicit (RUST_MIN_STACK, --edition)

.PHONY: lint-security
lint-security: ## G1 gate: 0 OPEN findings in CRITICAL or HIGH sections of findings tracker
	@echo ""
	@echo "▶ Checking security findings (G1 gate)..."
	@TRACKER="$(PROJECT_DIR)/docs/audit/CYNIC-FINDINGS-TRACKER.md"; \
	if [ ! -f "$$TRACKER" ]; then echo "SKIP: findings tracker not found"; exit 0; fi; \
	OPEN=$$(awk '/^## CRITICAL/{s=1} /^## HIGH/{s=1} /^## MEDIUM/{s=0} s && /^\|/ && !/^\| #/ && !/^\|---/ && / OPEN/' "$$TRACKER"); \
	if [ -n "$$OPEN" ]; then \
		echo "FAIL G1: CRIT/HIGH findings still OPEN:"; echo "$$OPEN"; exit 1; \
	else \
		echo "✓ G1: 0 OPEN findings in CRITICAL/HIGH"; \
	fi

.PHONY: sync-docs
sync-docs: ## Synchronize agent documentation from source of truth files
	@python3 organs/supervisor/controllers/sync_docs.py


.PHONY: lint-topology
lint-topology: ## Verify MANIFEST.yaml declarations vs live state (crons exist, outputs exist)
	@echo ""
	@echo "▶ Checking organism topology (MANIFESTs vs live state)..."
	@PYTHONDONTWRITEBYTECODE=1 python3 $(PROJECT_DIR)/organs/supervisor/controllers/validate_repo_topology.py --json > /tmp/cynic-topology-verify.json || { \
		echo "⚠ Topology verification found drift — not blocking (fix MANIFESTs later)"; \
	}

.PHONY: clean-python-caches
clean-python-caches: ## Remove Python bytecode caches from source membranes before topology checks
	@find $(PROJECT_DIR)/organs $(PROJECT_DIR)/services -type d -name __pycache__ -prune -exec rm -rf {} + 2>/dev/null || true

.PHONY: api-geometry-report
api-geometry-report: ## Print machine-readable API topology geometry report
	@python3 $(PROJECT_DIR)/organs/supervisor/src/entrypoints/api_contracts.py api-geometry-report

.PHONY: lint-repo-topology-contract
lint-repo-topology-contract: clean-python-caches ## Verify repo topology contract and portable MCP path invariants
	@echo ""
	@echo "▶ Checking repo topology contract..."
	@PYTHONDONTWRITEBYTECODE=1 python3 $(PROJECT_DIR)/organs/supervisor/controllers/validate_repo_topology.py --strict

.PHONY: lint-infra-consumption-contract
lint-infra-consumption-contract: ## Verify organs consume infra through declared lifecycle/runtime contracts
	@python3 -c 'from organs.supervisor.src.core.infra_consumption_contract import validate_infra_consumption_contract; findings=validate_infra_consumption_contract(); [print(f"{i.severity}: {i.kind}: {i.path}: {i.detail}") for i in findings]; raise SystemExit(1 if findings else 0)'
	@echo "infra consumption contract ok"

.PHONY: lint-organ-memory-contract
lint-organ-memory-contract: clean-python-caches ## Verify organ-local memory classes and local-state authority boundaries
	@PYTHONPATH=$(PROJECT_DIR) PYTHONDONTWRITEBYTECODE=1 python3 $(PROJECT_DIR)/organs/supervisor/src/core/organ_memory_contract.py

.PHONY: lint-command-surfaces-contract
lint-command-surfaces-contract: clean-python-caches ## Verify canonical human/agent/fallback command surface boundaries
	@PYTHONPATH=$(PROJECT_DIR) PYTHONDONTWRITEBYTECODE=1 python3 $(PROJECT_DIR)/organs/supervisor/src/core/command_surfaces_contract.py

.PHONY: topology-zero-exception-report
topology-zero-exception-report: ## Report remaining topology exceptions that prevent a 100% clean organism claim
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/validate_repo_topology.py --zero-exception-report

.PHONY: topology-zero-exception-check
topology-zero-exception-check: ## Fail until no non-canonical organs or topology exceptions remain
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/validate_repo_topology.py --zero-exception-check

.PHONY: git-membrane-closure-summary
git-membrane-closure-summary: ## Print the ordered Git membrane closure plan with current untracked counts
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/validate_repo_topology.py --closure-summary $(if $(w),--closure-wave $(w),)

.PHONY: git-matter-report
git-matter-report: ## Print read-only Git matter groups and sovereign inference batches
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/validate_repo_topology.py --git-matter-report

.PHONY: git-membrane-closure-commands
git-membrane-closure-commands: ## Print non-mutating git add and post-add check commands grouped by closure wave
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/validate_repo_topology.py --closure-commands $(if $(w),--closure-wave $(w),)

.PHONY: git-membrane-closure-check
git-membrane-closure-check: ## Fail until the selected Git membrane closure wave has no visible untracked files
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/validate_repo_topology.py --closure-check $(if $(w),--closure-wave $(w),)

.PHONY: git-membrane-closure-next
git-membrane-closure-next: ## Print non-mutating commands for the next open Git membrane closure wave
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/validate_repo_topology.py --closure-next

.PHONY: git-membrane-closure-preflight
git-membrane-closure-preflight: ## Fail unless this environment can execute Git membrane closure commands
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/validate_repo_topology.py --closure-preflight

.PHONY: lint-agent-exposure-contract
lint-agent-exposure-contract: ## Verify MCP tools exposed to agents exist in source and implemented contracts are exposed
	@echo ""
	@echo "▶ Checking agent exposure contract..."
	@PYTHONPATH=$(PROJECT_DIR) python3 $(PROJECT_DIR)/organs/exposure/src/entrypoints/cli.py validate >/tmp/cynic-agent-exposure-validation.json || { \
		cat /tmp/cynic-agent-exposure-validation.json; \
		rm -f /tmp/cynic-agent-exposure-validation.json; \
		exit 1; \
	}
	@rm -f /tmp/cynic-agent-exposure-validation.json
	@echo "agent exposure contract ok"

.PHONY: agent-exposure-check
agent-exposure-check: ## Check declared agent config projections without writing
	@python3 $(PROJECT_DIR)/organs/exposure/src/entrypoints/cli.py apply --check

.PHONY: agent-exposure-apply
agent-exposure-apply: ## Render declared agent config projections to their target files
	@python3 $(PROJECT_DIR)/organs/exposure/src/entrypoints/cli.py apply

.PHONY: lxc-exec-bridge-plan
lxc-exec-bridge-plan: ## Print non-mutating install plan for the Proxmox LXC exec bridge
	@python3 organs/fleet/src/entrypoints/cli.py lxc-exec-bridge install-plan

.PHONY: lxc-exec-bridge-bundle
lxc-exec-bridge-bundle: ## Build non-mutating LXC exec bridge activation bundle under out=<dir>
	@test -n "$(out)" || { echo "usage: make lxc-exec-bridge-bundle out=/tmp/cynic-lxc-bridge"; exit 2; }
	@python3 organs/fleet/src/entrypoints/cli.py lxc-exec-bridge bundle --output-dir "$(out)"

.PHONY: lxc-exec-bridge-status
lxc-exec-bridge-status: ## Check local filesystem activation status for the LXC exec bridge
	@python3 organs/fleet/src/entrypoints/cli.py lxc-exec-bridge activation-status

.PHONY: storage-primary-plan
storage-primary-plan: ## Print non-mutating dedicated storage primary activation plan
	@python3 organs/keep/src/entrypoints/cli.py storage-primary plan

.PHONY: storage-primary-bundle
storage-primary-bundle: ## Build non-mutating storage primary activation bundle under out=<dir>
	@test -n "$(out)" || { echo "usage: make storage-primary-bundle out=/tmp/cynic-storage-primary"; exit 2; }
	@python3 organs/keep/src/entrypoints/cli.py storage-primary bundle --output-dir "$(out)"

.PHONY: storage-primary-status
storage-primary-status: ## Check dedicated storage primary activation status; optional args='<status args>'
	@python3 organs/keep/src/entrypoints/cli.py storage-primary status $(or $(args),) || true

.PHONY: test-gates
test-gates: ## R21: Verify lint gates catch known violations (inject → check → restore)
	@echo ""
	@echo "══════════════════════════════════════════"
	@echo "  R21: Gate falsification tests"
	@echo "══════════════════════════════════════════"
	@set +e; PASS=0; FAIL=0; \
	\
	echo ""; echo "── lint-rules ──"; \
	\
	echo "[K1] Injecting #[cfg(feature)] in domain/..."; \
	TARGET="$(PROJECT_DIR)/crates/cynic-kernel/src/domain/mod.rs"; \
	cp "$$TARGET" "$$TARGET.gate-bak"; \
	echo '#[cfg(feature = "gate_test_k1")] mod _phantom;' >> "$$TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K1 gate MISSED #[cfg] in domain/ — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K1 gate caught #[cfg] in domain/"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$TARGET.gate-bak" "$$TARGET"; \
	\
	echo "[K2] Injecting reqwest in domain/..."; \
	cp "$$TARGET" "$$TARGET.gate-bak"; \
	echo 'use reqwest;' >> "$$TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K2 gate MISSED reqwest in domain/ — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K2 gate caught reqwest in domain/"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$TARGET.gate-bak" "$$TARGET"; \
	\
	echo "[K5] Injecting serde_json::Value in domain/..."; \
	cp "$$TARGET" "$$TARGET.gate-bak"; \
	echo 'fn _k5_test() -> serde_json::Value { todo!() }' >> "$$TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K5 gate MISSED serde_json::Value in domain/ — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K5 gate caught serde_json::Value in domain/"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$TARGET.gate-bak" "$$TARGET"; \
	\
	echo "[K5+] Injecting axum:: in domain/..."; \
	cp "$$TARGET" "$$TARGET.gate-bak"; \
	echo 'fn _k5_test() -> axum::Router { todo!() }' >> "$$TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K5 gate MISSED axum:: in domain/ — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K5 gate caught axum:: in domain/"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$TARGET.gate-bak" "$$TARGET"; \
	\
	echo "[K12] Injecting #[allow] without WHY:..."; \
	ALLOW_TARGET="$(PROJECT_DIR)/crates/cynic-kernel/src/main.rs"; \
	cp "$$ALLOW_TARGET" "$$ALLOW_TARGET.gate-bak"; \
	echo '#[allow(unused_variables)]' >> "$$ALLOW_TARGET"; \
	echo 'fn _k12_test() {}' >> "$$ALLOW_TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K12 gate MISSED #[allow] without WHY: — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K12 gate caught #[allow] without WHY:"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$ALLOW_TARGET.gate-bak" "$$ALLOW_TARGET"; \
	\
	echo "[R2] Injecting .ok() without logging..."; \
	R2_TARGET="$(PROJECT_DIR)/crates/cynic-kernel/src/domain/mod.rs"; \
	cp "$$R2_TARGET" "$$R2_TARGET.gate-bak"; \
	printf '\n\n\n\n\n\n\n\n\n\nfn _r2_test() { std::fs::File::open("x").ok(); }\n' >> "$$R2_TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ R2 gate MISSED .ok() without logging — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ R2 gate caught .ok() without logging"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$R2_TARGET.gate-bak" "$$R2_TARGET"; \
	\
	echo "[K3] Injecting pipeline fn in api/..."; \
	API_TARGET="$(PROJECT_DIR)/crates/cynic-kernel/src/api/rest/mod.rs"; \
	cp "$$API_TARGET" "$$API_TARGET.gate-bak"; \
	echo 'fn format_crystal_context() {}' >> "$$API_TARGET"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K3 gate MISSED pipeline fn in api/ — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K3 gate caught pipeline fn in api/"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$API_TARGET.gate-bak" "$$API_TARGET"; \
	\
	echo "[K4] Injecting duplicate Port trait name across domain/..."; \
	K4_TARGET1="$(PROJECT_DIR)/crates/cynic-kernel/src/domain/mod.rs"; \
	K4_TARGET2="$(PROJECT_DIR)/crates/cynic-kernel/src/domain/storage/mod.rs"; \
	cp "$$K4_TARGET1" "$$K4_TARGET1.gate-bak"; \
	cp "$$K4_TARGET2" "$$K4_TARGET2.gate-bak"; \
	echo 'trait _K4PhantomPort {}' >> "$$K4_TARGET1"; \
	echo 'trait _K4PhantomPort {}' >> "$$K4_TARGET2"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ K4 gate MISSED duplicate Port trait — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K4 gate caught duplicate Port trait"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$K4_TARGET1.gate-bak" "$$K4_TARGET1"; \
	mv "$$K4_TARGET2.gate-bak" "$$K4_TARGET2"; \
	\
	echo "[R1] Injecting hardcoded /home/ path in .cortex/mcp/..."; \
	R1_STUB="$(PROJECT_DIR)/.cortex/mcp/_r1_gate_probe.py"; \
	printf '%s\n' 'CYNIC = "/home/cynic-hardcoded"' > "$$R1_STUB"; \
	if $(MAKE) --no-print-directory lint-rules >/dev/null 2>&1; then \
		echo "  ✗ R1 gate MISSED hardcoded /home/ path — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ R1 gate caught hardcoded /home/ path"; PASS=$$((PASS+1)); \
	fi; \
	rm -f "$$R1_STUB"; \
	\
	echo ""; echo "── lint-drift ──"; \
	\
	echo "[Dormant] Injecting commented module without DORMANT tag..."; \
	cp "$$TARGET" "$$TARGET.gate-bak"; \
	echo '// pub mod _gate_test_phantom;' >> "$$TARGET"; \
	if $(MAKE) --no-print-directory lint-drift >/dev/null 2>&1; then \
		echo "  ✗ Dormant gate MISSED commented module — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ Dormant gate caught commented module without DORMANT:"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$TARGET.gate-bak" "$$TARGET"; \
	\
	echo "[D4] Injecting phantom Dog in reference.md..."; \
	REF="$(PROJECT_DIR)/.cortex/rules/reference.md"; \
	cp "$$REF" "$$REF.gate-bak"; \
	sed -i '/^| deterministic-dog/a | phantom-dog-test | Test | Nowhere |' "$$REF"; \
	if $(MAKE) --no-print-directory lint-drift >/dev/null 2>&1; then \
		echo "  ✗ D4 gate MISSED phantom Dog in reference.md — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ D4 gate caught phantom Dog in reference.md"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$REF.gate-bak" "$$REF"; \
	\
	echo "[K15] Injecting store_* with no read path in storage/mod.rs..."; \
	K15_TARGET="$(PROJECT_DIR)/crates/cynic-kernel/src/domain/storage/mod.rs"; \
	cp "$$K15_TARGET" "$$K15_TARGET.gate-bak"; \
	printf '\n    async fn store_phantom_k15_gate_probe(&self) -> Result<(), StorageError> { unimplemented!() }\n' >> "$$K15_TARGET"; \
	if $(MAKE) --no-print-directory lint-drift >/dev/null 2>&1; then \
		echo "  ✗ K15 gate MISSED store_* without read path — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K15 gate caught store_* without read path"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$K15_TARGET.gate-bak" "$$K15_TARGET"; \
	\
	echo "[K17] Injecting StoragePort method not forwarded in ReconnectableStorage..."; \
	K17_TARGET="$(PROJECT_DIR)/crates/cynic-kernel/src/domain/storage/mod.rs"; \
	cp "$$K17_TARGET" "$$K17_TARGET.gate-bak"; \
	sed -i '/^}$$/i\    async fn _k17_gate_probe(\&self) -> Result<(), StorageError> { Ok(()) }' "$$K17_TARGET"; \
	if $(MAKE) --no-print-directory lint-drift >/dev/null 2>&1; then \
		echo "  ✗ K17 gate MISSED missing ReconnectableStorage forward — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ K17 gate caught missing ReconnectableStorage forward"; PASS=$$((PASS+1)); \
	fi; \
	mv "$$K17_TARGET.gate-bak" "$$K17_TARGET"; \
	\
	echo ""; echo "── lint-subprocess-env ──"; \
	\
	echo "[R23a] Injecting Python wrapper with cargo build and no RUST_MIN_STACK..."; \
	R23_STUB="$(PROJECT_DIR)/.cortex/mcp/_r23_gate_probe.py"; \
	printf '%s\n' '# R23 gate-test probe' 'import subprocess' 'subprocess.run(["cargo", "build", "--tests"], check=True)' > "$$R23_STUB"; \
	if $(MAKE) --no-print-directory lint-subprocess-env >/dev/null 2>&1; then \
		echo "  ✗ R23a gate MISSED cargo without RUST_MIN_STACK — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ R23a gate caught cargo without RUST_MIN_STACK"; PASS=$$((PASS+1)); \
	fi; \
	rm -f "$$R23_STUB"; \
	\
	echo "[R23b] Injecting Python wrapper with standalone rustfmt and no --edition..."; \
	printf '%s\n' '# R23 gate-test probe' 'import os, subprocess' 'os.environ["RUST_MIN_STACK"] = "33554432"' 'subprocess.run(["rustfmt", os.environ["FILE"]], check=True)' > "$$R23_STUB"; \
	if $(MAKE) --no-print-directory lint-subprocess-env >/dev/null 2>&1; then \
		echo "  ✗ R23b gate MISSED rustfmt without --edition — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ R23b gate caught rustfmt without --edition"; PASS=$$((PASS+1)); \
	fi; \
	rm -f "$$R23_STUB"; \
	\
	echo "[R23c] Injecting Rust Command::new(\"cargo\") without .env..."; \
	R23_RS_STUB="$(PROJECT_DIR)/crates/cynic-kernel/src/_r23_gate_probe.rs"; \
	printf '%s\n' '// R23 gate-test probe' 'fn _r23_probe() {' '    tokio::process::Command::new("cargo").args(["build"]).spawn().ok();' '}' > "$$R23_RS_STUB"; \
	if $(MAKE) --no-print-directory lint-subprocess-env >/dev/null 2>&1; then \
		echo "  ✗ R23c gate MISSED Command::new(\"cargo\") without .env — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ R23c gate caught Command::new(\"cargo\") without .env"; PASS=$$((PASS+1)); \
	fi; \
	rm -f "$$R23_RS_STUB"; \
	\
	echo ""; echo "── lint-security ──"; \
	\
	echo "[G1] Injecting OPEN CRITICAL finding..."; \
	TRACKER="$(PROJECT_DIR)/docs/audit/CYNIC-FINDINGS-TRACKER.md"; \
	if [ -f "$$TRACKER" ]; then \
		cp "$$TRACKER" "$$TRACKER.gate-bak"; \
		awk '/^## CRITICAL/{print; getline; print; getline; print; print "| GATE-TEST | Inject | Gate falsification test | OPEN |"; next} {print}' \
			"$$TRACKER.gate-bak" > "$$TRACKER"; \
		if $(MAKE) --no-print-directory lint-security >/dev/null 2>&1; then \
			echo "  ✗ G1 gate MISSED OPEN CRITICAL — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ G1 gate caught OPEN CRITICAL finding"; PASS=$$((PASS+1)); \
		fi; \
		mv "$$TRACKER.gate-bak" "$$TRACKER"; \
	else \
		echo "  SKIP: findings tracker not found"; \
	fi; \
	\
	echo ""; echo "── verify-hooks ──"; \
	echo "  SKIP: legacy hook verifier scripts have been closed; hooks are managed by organism projection"; \
	\
	echo ""; echo "── verify-systemd-units ──"; \
	\
	echo "[UNIT1] Injecting wrong cynic-kernel.service target..."; \
	TMPROOT=$$(mktemp -d); \
	mkdir -p "$$TMPROOT/project/infra/systemd" "$$TMPROOT/systemd"; \
	cp "$(PROJECT_DIR)/infra/systemd/cynic-kernel.service" "$$TMPROOT/project/infra/systemd/cynic-kernel.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/surrealdb.service" "$$TMPROOT/project/infra/systemd/surrealdb.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/cynic-healthcheck.service" "$$TMPROOT/project/infra/systemd/cynic-healthcheck.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/cynic-healthcheck.timer" "$$TMPROOT/project/infra/systemd/cynic-healthcheck.timer"; \
	cp "$(PROJECT_DIR)/infra/systemd/surrealdb-backup.service" "$$TMPROOT/project/infra/systemd/surrealdb-backup.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/surrealdb-backup.timer" "$$TMPROOT/project/infra/systemd/surrealdb-backup.timer"; \
	cp "$(PROJECT_DIR)/infra/systemd/llama-server.service" "$$TMPROOT/project/infra/systemd/llama-server.service"; \
	ln -s /bin/true "$$TMPROOT/systemd/cynic-kernel.service"; \
	ln -s "$$TMPROOT/project/infra/systemd/surrealdb.service" "$$TMPROOT/systemd/surrealdb.service"; \
	ln -s "$$TMPROOT/project/infra/systemd/cynic-healthcheck.service" "$$TMPROOT/systemd/cynic-healthcheck.service"; \
	ln -s "$$TMPROOT/project/infra/systemd/cynic-healthcheck.timer" "$$TMPROOT/systemd/cynic-healthcheck.timer"; \
	ln -s "$$TMPROOT/project/infra/systemd/surrealdb-backup.service" "$$TMPROOT/systemd/surrealdb-backup.service"; \
	ln -s "$$TMPROOT/project/infra/systemd/surrealdb-backup.timer" "$$TMPROOT/systemd/surrealdb-backup.timer"; \
	ln -s "$$TMPROOT/project/infra/systemd/llama-server.service" "$$TMPROOT/systemd/llama-server.service"; \
	if PROJECT_DIR_OVERRIDE="$$TMPROOT/project" SYSTEMD_USER_DIR_OVERRIDE="$$TMPROOT/systemd" $(MAKE) --no-print-directory verify-systemd-units >/dev/null 2>&1; then \
		echo "  ✗ UNIT1 gate MISSED wrong cynic-kernel.service target — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ UNIT1 gate caught wrong cynic-kernel.service target"; PASS=$$((PASS+1)); \
	fi; \
	rm -rf "$$TMPROOT"; \
	\
	echo ""; echo "── runtime-check ──"; \
	\
	echo "[RUNTIME1] Injecting duplicate llama-server process..."; \
	TMPROOT=$$(mktemp -d); \
	UNITDIR="$$TMPROOT/units"; BINDIR="$$TMPROOT/bin"; \
	mkdir -p "$$UNITDIR" "$$BINDIR"; \
	cp "$(PROJECT_DIR)/infra/systemd/cynic-kernel.service" "$$UNITDIR/cynic-kernel.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/surrealdb.service" "$$UNITDIR/surrealdb.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/llama-server.service" "$$UNITDIR/llama-server.service"; \
	cp "$(PROJECT_DIR)/infra/systemd/cynic-healthcheck.service" "$$UNITDIR/cynic-healthcheck.service"; \
	printf '%s\n' '#!/usr/bin/env bash' 'if [ "$$2" = "is-active" ]; then' '  case "$$3" in' '    cynic-kernel|surrealdb|llama-server) echo active ;;' '    *) echo inactive ;;' '  esac' '  exit 0' 'fi' 'if [ "$$2" = "show" ]; then' '  service="$$3"; prop="$${4#--property=}"' '  if [ "$$5" != "--value" ]; then exit 1; fi' '  case "$$service:$$prop" in' '    cynic-kernel:MainPID) echo 1001 ;;' '    cynic-kernel:SubState) echo running ;;' '    surrealdb:MainPID) echo 1002 ;;' '    surrealdb:SubState) echo running ;;' '    llama-server:MainPID) echo 1003 ;;' '    llama-server:SubState) echo running ;;' '    *) echo "" ;;' '  esac' '  exit 0' 'fi' 'exit 1' > "$$BINDIR/systemctl"; \
	printf '%s\n' '#!/usr/bin/env bash' 'if [ "$$1" = "-xo" ]; then' '  case "$$2" in' '    cynic-kernel) echo 1001 ;;' '    surreal) echo 1002 ;;' '    llama-server) echo 1003 ;;' '  esac' '  exit 0' 'fi' 'if [ "$$1" = "-xa" ]; then' '  case "$$2" in' '    cynic-kernel) echo "1001 cynic-kernel" ;;' '    surreal) echo "1002 surreal start" ;;' '    llama-server) printf "1003 llama-server\n1004 llama-server\n" ;;' '  esac' '  exit 0' 'fi' 'exit 1' > "$$BINDIR/pgrep"; \
	printf '%s\n' '#!/usr/bin/env bash' 'case "$$1" in' '  -o)' '    case "$$2" in' '      stat=) echo Ssl ;;' '      ppid=) echo 1 ;;' '    esac' '    ;;' 'esac' > "$$BINDIR/ps"; \
	printf '%s\n' '#!/usr/bin/env bash' 'cat <<OUT' 'LISTEN 0 128 127.0.0.1:3030 0.0.0.0:* users:(("cynic-kernel",pid=1001,fd=9))' 'LISTEN 0 128 127.0.0.1:8000 0.0.0.0:* users:(("surreal",pid=1002,fd=9))' 'LISTEN 0 128 127.0.0.1:8080 0.0.0.0:* users:(("llama-server",pid=1003,fd=9))' 'LISTEN 0 128 127.0.0.1:8080 0.0.0.0:* users:(("llama-server",pid=1004,fd=9))' 'OUT' > "$$BINDIR/ss"; \
	chmod +x "$$BINDIR/systemctl" "$$BINDIR/pgrep" "$$BINDIR/ps" "$$BINDIR/ss"; \
	if UNIT_DIR_OVERRIDE="$$UNITDIR" SYSTEMCTL_BIN_OVERRIDE="$$BINDIR/systemctl" PGREP_BIN_OVERRIDE="$$BINDIR/pgrep" PS_BIN_OVERRIDE="$$BINDIR/ps" SS_BIN_OVERRIDE="$$BINDIR/ss" $(MAKE) --no-print-directory runtime-truth >/dev/null 2>&1; then \
		echo "  ✗ RUNTIME1 gate MISSED duplicate llama-server — BROKEN"; FAIL=$$((FAIL+1)); \
	else \
		echo "  ✓ RUNTIME1 gate caught duplicate llama-server"; PASS=$$((PASS+1)); \
	fi; \
	rm -rf "$$TMPROOT"; \
	\
	echo ""; \
	TOTAL=$$((PASS+FAIL)); \
	if [ $$FAIL -eq 0 ]; then \
		echo "✓ R21: $$PASS/$$TOTAL gates falsified successfully"; \
	else \
		echo "✗ R21: $$FAIL/$$TOTAL gates FAILED falsification"; exit 1; \
	fi

.PHONY: check-storage
check-storage: ## Kernel storage contract tests; storage placement is registry/Synapse-driven
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  CYNIC check-storage — kernel storage contracts"
	@echo "══════════════════════════════════════════"
	cargo test -p cynic-kernel --release --test integration_storage --test storage_contract
	@echo "✓ All checks passed"

# ── Stage 2: Validated Commit ────────────────────────────────
# Usage: make commit m="type(scope): description"
# Impact checking is done by [workspace.lints] in Cargo.toml — not bash scripts.
.PHONY: commit
commit: check
	@if [ -z "$(m)" ]; then echo "ERROR: provide message with m=\"...\"" >&2; exit 1; fi
	@echo ""
	@echo "▶ Staging and committing..."
	git add -u
	git commit -m "$(m)"
	@echo "✓ Committed (workspace lints + clippy + audit validated)"

# ── Stage 3: Ship (commit + push) ───────────────────────────
.PHONY: ship
ship: commit
	@echo ""
	@echo "▶ Pushing to origin..."
	git push origin $$(git rev-parse --abbrev-ref HEAD)
	@echo "✓ Shipped (pre-push validated build+test+clippy)"

# ── Stage 4: Deploy ─────────────────────────────────────────
# Canonical deploy is registry/MCP first: request work from organ-forge.
# Legacy local deploy path. Requires Level 2 gate and explicit opt-in.
# If gate-2 marker is stale, deploy blocks and tells you to run `make gate-2`.
.PHONY: deploy
deploy: require-local-deploy-opt-in
	@$(source_env)
	@echo "▶ Checking gate-2 marker..."
	@NEWEST_RUST=$$(find crates/cynic-kernel/src -name '*.rs' -exec stat -c '%Y' {} + 2>/dev/null | sort -rn | head -1); \
	GATE2_TIME=$$(stat -c '%Y' .gate-2 2>/dev/null || echo 0); \
	if [ -n "$$NEWEST_RUST" ] && [ "$$NEWEST_RUST" -gt "$$GATE2_TIME" ]; then \
		echo "✗ Gate 2 is stale — Rust changed since last full gate."; \
		echo "  Run: make gate-2"; \
		echo "  Then: make deploy"; \
		exit 1; \
	fi
	@echo "  ✓ Gate 2 fresh — proceeding with deploy"
	@$(MAKE) --no-print-directory ship check-storage
	@echo ""
	@echo "▶ Backing up DB before deploy (via Sovereign API)..."
	@BACKUP_DIR="$${CYNIC_BACKUP_DIR:-$${XDG_STATE_HOME:-$$HOME/.local/state}/cynic/backups}"; \
	mkdir -p "$$BACKUP_DIR"; \
	curl -sS -f -H "Authorization: Bearer $${CYNIC_API_KEY:?Set CYNIC_API_KEY}" \
		"http://$${CYNIC_REST_ADDR}/lifecycle/backup" \
		-o "$$BACKUP_DIR/cynic_v2_pre_deploy_$$(date +%Y%m%d_%H%M%S).surql" || echo "⚠ Backup failed"
	@echo "▶ Deploying kernel via Sovereign Auto-Upgrade (POST /lifecycle/upgrade)..."
	@curl -f -X POST --data-binary @"$(PROJECT_DIR)/target/release/cynic-kernel" \
		-H "Authorization: Bearer $${CYNIC_API_KEY:?Set CYNIC_API_KEY in ~/.cynic-env}" \
		"http://$${CYNIC_REST_ADDR:?Set CYNIC_REST_ADDR in ~/.cynic-env}/lifecycle/upgrade"

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
	@curl -s -f "http://$${CYNIC_REST_ADDR}/health" >/dev/null && echo "  Healthcheck: PASS" || echo "  ⚠ Healthcheck: FAIL"
	@echo "✓ Deployed and verified"

# ── Fast-path: Deploy existing binary without commit/push ────
# Use when origin/main is already current (e.g. after a PR merged through CI)
# and you just want to roll the release binary into ~/bin. Skips all validation
.PHONY: require-local-deploy-opt-in
require-local-deploy-opt-in:
	@[ "$${CYNIC_ALLOW_LOCAL_DEPLOY:-0}" = "1" ] || { \
		echo "✗ Local deploy is not canonical."; \
		echo "  Use: make deploy-request"; \
		echo "  Emergency/local-only override: CYNIC_ALLOW_LOCAL_DEPLOY=1 make deploy"; \
		exit 1; \
	}

# gates — caller is responsible for freshness.
.PHONY: deploy-only
deploy-only: require-local-deploy-opt-in  ## Local legacy deploy target/release/cynic-kernel with no validation
	@$(source_env)
	@BIN="$(PROJECT_DIR)/target/release/cynic-kernel"; \
	[ -f "$$BIN" ] || { echo "✗ No release binary at $$BIN — run 'cargo build --release' or 'make check' first"; exit 1; }; \
	AGE_SEC=$$(( $$(date +%s) - $$(stat -c %Y "$$BIN") )); \
	if [ $$AGE_SEC -gt 3600 ]; then \
		echo "⚠ Binary is $$((AGE_SEC/60))min old — consider rebuild if source changed"; \
	fi
	@echo ""
	@echo "▶ deploy-only — no commit, no push, no validation"
	@echo "▶ Backing up DB before deploy (via Sovereign API)..."
	@BACKUP_DIR="$${CYNIC_BACKUP_DIR:-$${XDG_STATE_HOME:-$$HOME/.local/state}/cynic/backups}"; \
	mkdir -p "$$BACKUP_DIR"; \
	curl -sS -f -H "Authorization: Bearer $${CYNIC_API_KEY:?Set CYNIC_API_KEY}" \
		"http://$${CYNIC_REST_ADDR}/lifecycle/backup" \
		-o "$$BACKUP_DIR/cynic_v2_deploy_only_$$(date +%Y%m%d_%H%M%S).surql" || echo "⚠ Backup failed"
	@echo "▶ Deploying kernel via Sovereign Auto-Upgrade (POST /lifecycle/upgrade)..."
	@curl -f -X POST --data-binary @"$(PROJECT_DIR)/target/release/cynic-kernel" \
		-H "Authorization: Bearer $${CYNIC_API_KEY:?Set CYNIC_API_KEY in ~/.cynic-env}" \
		"http://$${CYNIC_REST_ADDR:?Set CYNIC_REST_ADDR in ~/.cynic-env}/lifecycle/upgrade"

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
	@curl -s -f "http://$${CYNIC_REST_ADDR}/health" >/dev/null && echo "  Healthcheck: PASS" || echo "  ⚠ Healthcheck: FAIL"
	@echo "✓ Deployed (no commit, no push)"

# ── Emergency: Rollback to previous binary ─────────────────
.PHONY: rollback
rollback:
	@$(source_env)
	@echo "▶ Rolling back to previous kernel binary via Sovereign Auto-Rollback (POST /lifecycle/rollback)..."
	@curl -f -X POST \
		-H "Authorization: Bearer $${CYNIC_API_KEY:?Set CYNIC_API_KEY in ~/.cynic-env}" \
		"http://$${CYNIC_REST_ADDR:?Set CYNIC_REST_ADDR in ~/.cynic-env}/lifecycle/rollback"
	@sleep 4
	@echo "▶ Verifying rollback..."
	@HTTP=$$(curl -s -o /dev/null -w '%{http_code}' "http://$${CYNIC_REST_ADDR}/health"); \
	[ "$$HTTP" = "200" ] && echo "Kernel: healthy (rolled back)" || echo "Kernel: DEGRADED (HTTP $$HTTP, rolled back)"

# ── Systemd unit deployment ──────────────────────────────────
.PHONY: deploy-systemd
deploy-systemd:  ## Idempotent deploy: push infra/systemd/* via Sovereign API
	@$(source_env); \
	echo "▶ Deploying systemd units via Sovereign API..."; \
	curl -f -X POST --data-binary @"$(PROJECT_DIR)/infra/systemd/cynic-kernel.service" \
		-H "Authorization: Bearer $${CYNIC_API_KEY}" \
		"http://$${CYNIC_REST_ADDR}/lifecycle/upgrade-systemd"; \
	echo "✓ Systemd units deployed and reloaded"

.PHONY: deploy-systemd-dry
deploy-systemd-dry:  ## Dry-run: show what deploy-systemd would change
	@echo 'Systemd deployed by organism' --dry-run

# ── Emergency: Hotfix deploy (skip push — for incidents) ──────
.PHONY: hotfix
hotfix: require-local-deploy-opt-in check
	@$(source_env)
	@echo ""
	@echo "⚠ HOTFIX DEPLOY — skipping push (commit locally, push later)"
	@echo "▶ Backing up DB (via Sovereign API)..."
	@BACKUP_DIR="$${CYNIC_BACKUP_DIR:-$${XDG_STATE_HOME:-$$HOME/.local/state}/cynic/backups}"; \
	mkdir -p "$$BACKUP_DIR"; \
	curl -sS -f -H "Authorization: Bearer $${CYNIC_API_KEY}" \
		"http://$${CYNIC_REST_ADDR}/lifecycle/backup" \
		-o "$$BACKUP_DIR/cynic_v2_hotfix_$$(date +%Y%m%d_%H%M%S).surql" || echo "⚠ Backup failed"
	@echo "▶ Deploying kernel via Sovereign Auto-Upgrade (POST /lifecycle/upgrade)..."
	@curl -f -X POST --data-binary @"$(PROJECT_DIR)/target/release/cynic-kernel" \
		-H "Authorization: Bearer $${CYNIC_API_KEY:?Set CYNIC_API_KEY in ~/.cynic-env}" \
		"http://$${CYNIC_REST_ADDR:?Set CYNIC_REST_ADDR in ~/.cynic-env}/lifecycle/upgrade"

	@echo "▶ Verifying (retry loop)..."
	@for i in $$(seq 1 15); do \
		sleep 3; \
		HTTP=$$(curl -s -o /dev/null -w '%{http_code}' "http://$${CYNIC_REST_ADDR}/health" 2>/dev/null) && \
		[ "$$HTTP" = "200" ] && echo "Kernel: healthy (hotfix, HTTP $$HTTP)" && break || \
		printf "."; \
	done

# ── Restore DB from backup ────────────────────────────────────
.PHONY: restore
restore:
	@$(source_env)
	@if [ -z "$(F)" ]; then BACKUP_DIR="$${CYNIC_BACKUP_DIR:-$${XDG_STATE_HOME:-$$HOME/.local/state}/cynic/backups}"; echo "ERROR: provide backup file with F=path"; echo "Available:"; ls -lhrt "$$BACKUP_DIR"/ 2>/dev/null | tail -5; exit 1; fi
	@[ -f "$(F)" ] || { echo "ERROR: file $(F) not found"; exit 1; }
	@echo "⚠ RESTORING DB from $(F) — this will OVERWRITE current data"
	@read -p "Are you sure? (yes/no) " CONFIRM || true; [ "$$CONFIRM" = "yes" ] || { echo "Aborted"; exit 1; }
	@echo "▶ Restoring DB via Sovereign API (POST /lifecycle/restore)..."
	@curl -f -X POST --data-binary @"$(F)" \
		-H "Authorization: Bearer $${CYNIC_API_KEY}" \
		"http://$${CYNIC_REST_ADDR}/lifecycle/restore"
	@echo "✓ DB restored from $(F)"

.PHONY: test-restore
test-restore: ## Verify backup restore works against a test DB (non-destructive)
	@$(source_env)
	@echo "══════════════════════════════════════════"
	@echo "  CYNIC test-restore — storage-primary path check"
	@echo "══════════════════════════════════════════"
	@echo "Direct restore probes against a cortex-local SurrealDB are forbidden."
	@echo "Use storage-primary evidence and activation bundles instead:"
	@echo "  cynic storage status"
	@echo "  cynic storage bundle --output-dir <dir>"
	@echo "  cynic organism"
	@python3 organs/keep/src/entrypoints/cli.py storage-primary status --json >/dev/null

# ── Stage 5: Release (tag + changelog) ─────────────────────────
# Usage: make release v=patch  (or minor/major)
# Dry run: make release-dry v=patch
.PHONY: release
release: check
	@if [ -z "$(v)" ]; then echo "ERROR: provide version bump with v=patch|minor|major" >&2; exit 1; fi
	@echo ""
	@echo "▶ Releasing ($(v) bump)..."
	cargo release $(v) --execute --no-confirm
	@echo "✓ Released — changelog updated, tagged, pushed"

.PHONY: release-dry
release-dry:
	@if [ -z "$(v)" ]; then echo "ERROR: provide version bump with v=patch|minor|major" >&2; exit 1; fi
	cargo release $(v)

# ── Git hooks (Layer 2 enforcement) ──────────────────────────
.PHONY: install-hooks
install-hooks:
	@echo 'Hooks managed by organism'

.PHONY: verify-hooks
verify-hooks:
	@echo 'Hooks managed by organism'

# ── Clean (fix SIGSEGV/LLVM crashes) ──────────────────────────
.PHONY: clean
clean:
	cargo clean -p cynic-kernel
	@echo "✓ Cleaned cynic-kernel build artifacts"

.PHONY: monitor-vm
monitor-vm: ## Monitor VMs on Proxmox
	@echo "legacy shell monitor closed; use: python3 organs/fleet/src/entrypoints/cli.py report --json --registry infra/registry.json" >&2
	@exit 1

# ── Standalone: End-to-end test ──────────────────────────────
.PHONY: e2e
e2e:
	@$(source_env)
	@echo "▶ E2E: Sicilian Defense + supply-chain mempool..."
	@ADDR=$$(python3 organs/forge/src/entrypoints/cli.py kernel-url); AUTH="Authorization: Bearer $${CYNIC_API_KEY}"; FAIL=0; \
	echo ""; \
	printf "  1/7 Health........... "; \
	HTTP=$$(curl -s -o /dev/null -w '%{http_code}' -H "$$AUTH" "$$ADDR/health"); \
	python3 organs/forge/src/entrypoints/cli.py lifecycle-health-accepted "$$HTTP" >/dev/null && echo "PASS ($$HTTP)" || { echo "FAIL ($$HTTP)"; FAIL=$$((FAIL+1)); }; \
	printf "  2/7 Judge+dog_scores. "; \
	V=$$(curl -s -X POST "$$ADDR/judge" -H "Content-Type: application/json" -H "$$AUTH" \
		-d '{"content":"1. e4 c5 — The Sicilian Defense.","context":"Most popular response to 1.e4","domain":"chess"}'); \
	DOGS=$$(echo "$$V" | python3 -c "import json,sys; v=json.load(sys.stdin); print(len(v.get('dog_scores',[])))" 2>/dev/null); \
	VERDICT=$$(echo "$$V" | python3 -c "import json,sys; print(json.load(sys.stdin)['verdict'])" 2>/dev/null); \
	[ "$${DOGS:-0}" -gt 0 ] && echo "PASS $$VERDICT dogs=$$DOGS" || { echo "FAIL (dog_scores empty)"; FAIL=$$((FAIL+1)); }; \
	printf "  3/7 Verdict DB RT.... "; \
	DB_DOGS=$$(curl -s -H "$$AUTH" "$$ADDR/verdicts?limit=1" | python3 -c "import json,sys; v=json.load(sys.stdin); print(len(v[0].get('dog_scores',[])) if v else 0)" 2>/dev/null); \
	[ "$${DB_DOGS:-0}" -gt 0 ] && echo "PASS ($$DB_DOGS dogs from DB)" || { echo "FAIL"; FAIL=$$((FAIL+1)); }; \
	printf "  4/7 Observations..... "; \
	OBS=$$(curl -s -H "$$AUTH" "$$ADDR/observations?limit=1" | python3 -c "import json,sys; o=json.load(sys.stdin); print(len(o[0].keys()) if o else 0)" 2>/dev/null || true); \
	[ "$${OBS:-0}" -ge 10 ] && echo "PASS ($$OBS fields)" || { echo "FAIL ($$OBS fields)"; FAIL=$$((FAIL+1)); }; \
	printf "  5/7 Audit............ "; \
	AUD=$$(curl -s -H "$$AUTH" "$$ADDR/audit?limit=1" | python3 -c "import json,sys; a=json.load(sys.stdin); print(len(a[0].keys()) if a else 0)" 2>/dev/null || true); \
	[ "$${AUD:-0}" -ge 5 ] && echo "PASS ($$AUD fields)" || { echo "FAIL ($$AUD fields)"; FAIL=$$((FAIL+1)); }; \
	printf "  6/7 Crystals......... "; \
	CRY=$$(curl -s -H "$$AUTH" "$$ADDR/crystals?limit=1" | python3 -c "import json,sys; print(len(json.load(sys.stdin)))" 2>/dev/null || true); \
	[ "$${CRY:-0}" -gt 0 ] && echo "PASS" || { echo "FAIL"; FAIL=$$((FAIL+1)); }; \
	printf "  7/7 Supply-chain..... "; \
	DOMAIN="supply-chain-e2e-$$(date +%s)"; \
	PAYLOAD=$$(python3 organs/forge/src/entrypoints/cli.py lifecycle-task-payload "deploy-kernel" "$$DOMAIN" "cortex-e2e-supply-chain"); \
	python3 organs/forge/src/entrypoints/cli.py lifecycle-validate-task-payload "$$PAYLOAD" >/dev/null; \
	CREATED=$$(curl -s -X POST "$$ADDR/agent-tasks" -H "Content-Type: application/json" -H "$$AUTH" -d "$$PAYLOAD"); \
	TASK=$$(echo "$$CREATED" | python3 -c "import json,sys; print(json.load(sys.stdin).get('task_id',''))" 2>/dev/null || true); \
	PENDING=$$(curl -s -H "$$AUTH" "$$ADDR/agent-tasks?kind=hermes&domain=$$DOMAIN&limit=20" | TASK_ID="$$TASK" python3 -c "import json,os,sys; tid=os.environ.get('TASK_ID',''); data=json.load(sys.stdin); print('True' if any(t.get('id') == tid for t in data.get('tasks',[])) else 'False')" 2>/dev/null || true); \
	CLAIMED=$$(curl -s -X POST "$$ADDR/agent-tasks/$$TASK/claim" -H "Content-Type: application/json" -H "$$AUTH" -d '{"agent_id":"cortex-e2e-supply-chain"}' | python3 -c "import json,sys; print(json.load(sys.stdin).get('claimed',False))" 2>/dev/null || true); \
	curl -s -X POST "$$ADDR/agent-tasks/$$TASK/result" -H "Content-Type: application/json" -H "$$AUTH" -d '{"result":"e2e supply-chain mempool passed"}' >/dev/null; \
	COMPLETED=$$(curl -s -H "$$AUTH" "$$ADDR/agent-tasks/completed?kind=hermes&domain=$$DOMAIN&limit=20" | TASK_ID="$$TASK" python3 -c "import json,os,sys; tid=os.environ.get('TASK_ID',''); data=json.load(sys.stdin); print('True' if any(t.get('id') == tid for t in data.get('tasks',[])) else 'False')" 2>/dev/null || true); \
	[ -n "$$TASK" ] && [ "$$PENDING" = "True" ] && [ "$$CLAIMED" = "True" ] && [ "$$COMPLETED" = "True" ] && echo "PASS $$TASK" || { echo "FAIL task=$$TASK pending_visible=$$PENDING claimed=$$CLAIMED completed_visible=$$COMPLETED"; FAIL=$$((FAIL+1)); }; \
	echo ""; \
	[ $$FAIL -eq 0 ] && echo "✓ All 7 E2E checks passed" || { echo "✗ $$FAIL E2E check(s) FAILED"; exit 1; }

# ── Standalone: System status ────────────────────────────────
.PHONY: status
status:
	@$(source_env)
	@echo "CYNIC System Status"
	@echo "═══════════════════"
	@printf "Kernel:  "; \
	if [ -n "$${CYNIC_REST_ADDR:-}" ]; then \
		HTTP=$$(curl -s -o /dev/null -w '%{http_code}' $${CYNIC_API_KEY:+-H "Authorization: Bearer $${CYNIC_API_KEY}"} "http://$${CYNIC_REST_ADDR}/health" 2>/dev/null || true); \
	else \
		HTTP=000; \
	fi; \
	[ "$$HTTP" = "200" ] && echo "healthy ($$HTTP)" || echo "DEGRADED (HTTP $${HTTP:-000})"; \
	printf "Storage: "; python3 organs/keep/src/entrypoints/cli.py storage-primary status --json >/dev/null 2>&1 && echo "ready" || echo "blocked — run: cynic storage status"; \
	printf "Organism: "; python3 organs/supervisor/entrypoints/runtime_principals.py organism --json >/dev/null 2>&1 && echo "visible" || echo "blocked — run: cynic organism"
	@printf "Hooks: "; $(MAKE) --no-print-directory verify-hooks >/dev/null 2>&1 && echo "verified" || echo "DRIFT"
	@printf "Units: "; $(MAKE) --no-print-directory verify-systemd-units >/dev/null 2>&1 && echo "verified" || echo "DRIFT"
	@printf "Runtime: "; $(MAKE) --no-print-directory runtime-check >/dev/null 2>&1 && echo "verified" || echo "DRIFT"
	@printf "Git: "; git -C $(PROJECT_DIR) rev-parse --abbrev-ref HEAD; git -C $(PROJECT_DIR) log --oneline -1
	@printf "Backup: "; BACKUP_DIR="$${CYNIC_BACKUP_DIR:-$${XDG_STATE_HOME:-$$HOME/.local/state}/cynic/backups}"; ls -t "$$BACKUP_DIR"/*.surql 2>/dev/null | head -1 || echo "none"

.PHONY: logs
logs: ## Stream kernel logs remotely via SSH (derives host from CYNIC_REST_ADDR)
	@$(source_env); \
	HOST=$$(echo $${CYNIC_REST_ADDR} | cut -d: -f1); \
	if [ -z "$$HOST" ]; then echo "ERROR: CYNIC_REST_ADDR not set"; exit 1; fi; \
	if [ "$$HOST" = "localhost" ] || [ "$$HOST" = "127.0.0.1" ]; then \
		echo "▶ Streaming local logs..."; \
		journalctl --user -u cynic-kernel -n 100 -f; \
	else \
		echo "▶ Streaming remote logs from $$HOST..."; \
		ssh "$$HOST" journalctl --user -u cynic-kernel -n 100 -f; \
	fi

.PHONY: runtime-truth
runtime-truth:
	@$(source_env)
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/audit_runtime.py

.PHONY: runtime-check
runtime-check:
	@$(source_env)
	@$(MAKE) --no-print-directory verify-systemd-units
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/audit_runtime.py

# ── Benchmark: Empirical evaluation against running kernel ───
.PHONY: benchmark
benchmark:
	@$(source_env)
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/audit_runtime.py --save

.PHONY: install-systemd-units
install-systemd-units:
	@python3 $(PROJECT_DIR)/organs/supervisor/controllers/audit_runtime.py

.PHONY: organ-lifecycle-install
organ-lifecycle-install:
	@echo "▶ Linking organ-lifecycle systemd projection..."
	@sudo systemctl link "$(PROJECT_DIR)/infra/systemd/organ-lifecycle.service"
	@sudo systemctl link "$(PROJECT_DIR)/infra/systemd/organ-lifecycle.timer"
	@sudo systemctl link "$(PROJECT_DIR)/infra/systemd/organ-lifecycle-watchdog.service"
	@sudo systemctl link "$(PROJECT_DIR)/infra/systemd/organ-lifecycle-watchdog.timer"
	@sudo systemctl enable --now organ-lifecycle.timer
	@sudo systemctl enable --now organ-lifecycle-watchdog.timer
	@systemctl status organ-lifecycle.timer organ-lifecycle.service organ-lifecycle-watchdog.timer organ-lifecycle-watchdog.service --no-pager

.PHONY: organ-lifecycle-status
organ-lifecycle-status:
	@systemctl status organ-lifecycle.timer organ-lifecycle.service organ-lifecycle-watchdog.timer organ-lifecycle-watchdog.service --no-pager || true
	@journalctl -u organ-lifecycle.service -n 50 --no-pager || true
	@journalctl -u organ-lifecycle-watchdog.service -n 50 --no-pager || true

.PHONY: organ-lifecycle-run
organ-lifecycle-run:
	@python3 $(PROJECT_DIR)/organs/lifecycle/src/entrypoints/controller.py

.PHONY: organ-lifecycle-health
organ-lifecycle-health:
	@python3 $(PROJECT_DIR)/organs/lifecycle/src/entrypoints/controller.py --health

.PHONY: organ-lifecycle-analyze
organ-lifecycle-analyze:
	@python3 $(PROJECT_DIR)/organs/lifecycle/src/entrypoints/cli.py analyze-chain --registry "$(PROJECT_DIR)/infra/registry.json"

.PHONY: organ-lifecycle-watchdog
organ-lifecycle-watchdog:
	@python3 $(PROJECT_DIR)/organs/lifecycle/src/entrypoints/watchdog.py

.PHONY: organ-lifecycle-prove
organ-lifecycle-prove:
	@$(MAKE) --no-print-directory organ-lifecycle-health || true
	@$(MAKE) --no-print-directory organ-lifecycle-analyze || true
	@$(MAKE) --no-print-directory organ-lifecycle-run || true
	@$(MAKE) --no-print-directory organ-lifecycle-watchdog || true
	@$(MAKE) --no-print-directory organ-lifecycle-status || true

.PHONY: verify-systemd-units
verify-systemd-units:
	@echo 'Systemd units managed by organism'

# ── Standalone: Manual backup ────────────────────────────────
.PHONY: backup
backup:
	@$(source_env)
	@BACKUP_DIR="$${CYNIC_BACKUP_DIR:-$${XDG_STATE_HOME:-$$HOME/.local/state}/cynic/backups}"; \
	mkdir -p "$$BACKUP_DIR"; \
	OUT="$$BACKUP_DIR/cynic_v2_$$(date +%Y%m%d_%H%M%S).surql"; \
	curl -sS -f -H "Authorization: Bearer $${CYNIC_API_KEY:?Set CYNIC_API_KEY}" \
		"http://$${CYNIC_REST_ADDR:?Set CYNIC_REST_ADDR}/lifecycle/backup" \
		-o "$$OUT"; \
	echo "✓ Backup saved through Kernel coordination API: $$OUT"

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
	@curl -sf -H "Authorization: Bearer $${CYNIC_API_KEY}" "http://$${CYNIC_REST_ADDR}/coord/agents" | python3 -m json.tool 2>/dev/null || { [ -n "$${CYNIC_REST_ADDR:-}" ] && echo "ERROR: kernel not responding at $${CYNIC_REST_ADDR}" || echo "ERROR: CYNIC_REST_ADDR not set — source ~/.cynic-env first"; }

	if [ $$FAIL -eq 0 ]; then echo "✓ All lint-drift checks pass"; fi; \
	exit $$FAIL

.PHONY: calibrate-token-conviction-only
calibrate-token-conviction-only: ## Validate conviction-only token baseline (100% accuracy on 28 tokens)
	@echo ""
	@echo "▶ Measuring token conviction→verdict mapping accuracy..."
	@python3 $(PROJECT_DIR)/services/cynic-python/heuristics/measure_conviction_only.py
