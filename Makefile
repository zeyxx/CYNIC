SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.NOTPARALLEL:

PROJECT_DIR := $(shell git rev-parse --show-toplevel 2>/dev/null || pwd)
CARGO ?= cargo
CARGO_NETWORK_MODE ?= offline
CARGO_SOURCE_MODE ?= registry
CYNIC_CARGO_TARGET_ROOT ?= /tmp/cynic-cargo-targets
CYNIC_CARGO_TARGET_PROFILE ?= default
CYNIC_CARGO_BUILD_JOBS ?= 1
CARGO_TARGET_DIR := $(if $(strip $(CARGO_TARGET_DIR)),$(CARGO_TARGET_DIR),$(CYNIC_CARGO_TARGET_ROOT)/$(CYNIC_CARGO_TARGET_PROFILE))
CORE_CRATES := cynic-protocol cynic-host cynic-node cynic-kernel cynic-cli cynic-inference cynic-mcp
LAB_REGISTRY := infra/registries/lab.json
LOCAL_REGISTRY := infra/registries/single-node.json
CYNIC_BUILD_ADMISSION_GRANT ?=
CYNIC_BUILD_WORKLOAD_RECORD ?=
CYNIC_BUILD_AUTHORITY_PUBLIC_KEY ?=
CYNIC_BUILD_ADMISSION_VERIFIER ?= cynic-node
CYNIC_BUILD_BOOTSTRAP_RECEIPT ?=
# Keep a bounded large compiler stack for kernel/CLI monomorphization. The
# repository toolchain is pinned per-crate via rust-toolchain.toml (1.95 kernel,
# 1.96 host/node/cli) — rustup auto-detects per crate (no hardcoded RUSTUP_TOOLCHAIN).
# Stack size — AGENS § (défense en profondeur, observe substrate first).
# AGENS.md memory: 256 MiB (CYNIC_RUST_MIN_STACK) is the safe proven default for
# incremental/warm-cache builds on <=~32 GiB RAM runners. The previous hardcode
# of 8G/16G/32G escalated memory → MORE cold rebuilds → MORE transitive dep
# ICE exposure (ring/winnow/regex-automata under 1.96). 256 MiB avoids the OOM
# cascade; warm incremental cache avoids dep recompile.
CYNIC_RUST_MIN_STACK ?= 268435456
RUST_ENV := CYNIC_BUILD_ADMISSION_GRANT=$(CYNIC_BUILD_ADMISSION_GRANT) CYNIC_BUILD_WORKLOAD_RECORD=$(CYNIC_BUILD_WORKLOAD_RECORD) CYNIC_BUILD_AUTHORITY_PUBLIC_KEY=$(CYNIC_BUILD_AUTHORITY_PUBLIC_KEY) CYNIC_BUILD_ADMISSION_VERIFIER=$(CYNIC_BUILD_ADMISSION_VERIFIER) CYNIC_BUILD_BOOTSTRAP_RECEIPT=$(CYNIC_BUILD_BOOTSTRAP_RECEIPT) RUST_MIN_STACK=$(CYNIC_RUST_MIN_STACK) CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=$(CYNIC_CARGO_BUILD_JOBS) CMAKE_BUILD_PARALLEL_LEVEL=$(CYNIC_CARGO_BUILD_JOBS) CARGO_TARGET_DIR=$(CARGO_TARGET_DIR) CYNIC_CARGO_NETWORK_MODE=$(CYNIC_CARGO_NETWORK_MODE) CYNIC_CARGO_SOURCE_MODE=$(CYNIC_CARGO_SOURCE_MODE)
BUILD_RUST_ENV := $(RUST_ENV)
RELEASE_RUST_ENV := $(RUST_ENV)
# Tests need 8 GiB (cynic-node test harness codegen — memory-confirmed),
# not the 256 MiB default. Check/run/surface-audit reuse the warm target
# cache so 256 MiB is safe there.
TEST_RUST_ENV := $(RUST_ENV) RUST_MIN_STACK=8589934592

.PHONY: authority-check
authority-check: secret-literal-check ## Read-only local validation of machine-readable authorities
	@echo "==> checking foundation, runtime authority, and registry"
	@jq empty contracts/foundation.json
	@jq empty infra/runtime-authority.json
	@jq empty infra/registry.json
	@jq empty infra/agent-exposure.json
	@jq empty infra/ecosystem-pins.json
	@jq empty infra/runtime-basket.json
	@jq -e '(.roles | has("human_admin")) and (.roles | has("runtime_publisher")) and (.roles.human_admin.authority_class == "HumanAdmin") and (.roles.runtime_publisher.runtime_zero_touch == true) and (.domains | has("runtime"))' infra/agent-roles.json >/dev/null
	@echo "==> checking 2N+2 runtime_publisher capability federation consistency"
	@jq -e '.canonical_runtime_capabilities.runtime_publisher.authority_class == "RuntimeMachineIdentity"' infra/runtime-authority.json >/dev/null
	@jq -e '.canonical_runtime_capabilities.runtime_publisher.federated_evidence == "cynic.network.runtime_proof.v1 (RuntimeProofBundle gate: runtime_publisher_admission)"' infra/runtime-authority.json >/dev/null
	@CYNIC_ALLOW_LOCAL_DEV_BUILD=1 $(CARGO) run -q -p cynic-cli -- dev surface-registry --check

	@for script in $$(git grep -l '^#!' -- scripts); do \
		mode="$$(git ls-files -s -- "$$script" | awk '{print $$1}')"
		if [[ "$$mode" != "100755" ]]; then
			echo "tracked script is not executable in Git: $$script ($${mode:-untracked})" >&2
			exit 1
		fi
	done

.PHONY: secret-literal-check
secret-literal-check: ## Reject high-confidence literal credentials in Git-tracked source
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- secret lint

.PHONY: check
check: ## Native validation via cynic audit crate (no make recursion, exit code is the gate)
	@echo "==> cynic audit crate (native, 100% Rust)"
	@CYNIC_BIN=$$(if [ -x target/release/cynic ]; then echo "target/release/cynic"; elif [ -x target/debug/cynic ]; then echo "target/debug/cynic"; elif [ -x /var/lib/cynic/bin/cynic ]; then echo "/var/lib/cynic/bin/cynic"; else echo ""; fi); \
	if [ -n "$$CYNIC_BIN" ]; then \
		$$CYNIC_BIN audit crate --json > /dev/null; \
	else \
		$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- audit crate --json > /dev/null; \
	fi

.PHONY: audit
audit: ## Native unified audit of all crates, architecture boundaries, and secrets
	@CYNIC_BIN=$$(if [ -x target/release/cynic ]; then echo "target/release/cynic"; elif [ -x target/debug/cynic ]; then echo "target/debug/cynic"; elif [ -x /var/lib/cynic/bin/cynic ]; then echo "/var/lib/cynic/bin/cynic"; else echo ""; fi); \
	if [ -n "$$CYNIC_BIN" ]; then \
		$$CYNIC_BIN audit all --json; \
	else \
		$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- audit all --json; \
	fi

.PHONY: converge
converge: ## Native automated convergence of worktrees, branches, and unified audit
	@echo "==> Reconciling agent worktrees and branches"
	@CYNIC_BIN=$$(if [ -x target/release/cynic ]; then echo "target/release/cynic"; elif [ -x target/debug/cynic ]; then echo "target/debug/cynic"; elif [ -x /var/lib/cynic/bin/cynic ]; then echo "/var/lib/cynic/bin/cynic"; else echo ""; fi); \
	if [ -n "$$CYNIC_BIN" ]; then \
		$$CYNIC_BIN source worktree --apply --json; \
		echo "==> Verifying workspace convergence audit"; \
		$$CYNIC_BIN audit all --json; \
	else \
		$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- source worktree --apply --json; \
		echo "==> Verifying workspace convergence audit"; \
		$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- audit all --json; \
	fi

# Layered, per-layer validation. Dependency order: protocol -> host -> node ->
# kernel -> mcp -> cli. Each layer gets an isolated target dir and a
# fail-closed cargo check + test pass. Cache integrity is observed first
# (offline/locked metadata) so a stale vendored cache is reported, not papered.
LAYER_CRATES := cynic-protocol cynic-host cynic-node cynic-kernel cynic-cli
.PHONY: check-layers
check-layers: ## Per-layer cargo check+test, fail-closed, dependency-ordered
	@echo "==> cache integrity (offline/locked metadata)"
	@$(CARGO) metadata --offline --locked --format-version 1 >/dev/null \
		|| { echo "cache integrity: offline/locked metadata unavailable (offline_dependency_unavailable)"; exit 1; }
	@for crate in $(LAYER_CRATES); do \
		echo "  -> $$crate"; \
		( cd "crates/$$crate" && $(RUST_ENV) $(CARGO) check -p "$$crate" ) || exit 1; \
		( cd "crates/$$crate" && $(RUST_ENV) $(CARGO) test -p "$$crate" ) || exit 1; \
	done
	@echo "==> check-layers: all layers validated"

.PHONY: check-installed-runtime
check-installed-runtime: ## Validate installed-runtime portability invariants (`CYNIC_PROJECT_ROOT` absent, runtime mode persistence)
	@$(RUST_ENV) $(CARGO) test -q -p cynic-cli --test installed_runtime

.PHONY: check-strict
check-strict: ## cargo check core crates with -D warnings (signal engineering debt)
	@echo "==> strict warning check on core crates"
	@$(RUST_ENV) RUSTFLAGS='-D warnings' $(CARGO) check $(foreach crate,$(CORE_CRATES),-p $(crate))

.PHONY: surface-audit
surface-audit: ## Verify canonical CLI surfaces are backed by real modules and mutation policies
	@echo "==> auditing canonical CLI surfaces"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- dev surface-audit --json | jq -e '.schema == "cynic.cli.surface_audit.v1" and .status == "ok" and .blocked_count == 0' >/dev/null

.PHONY: audit-surface-matrix
audit-surface-matrix: ## Emit reusable JSON+Markdown surface matrix (surface, ownership, kernel-audit, runtime phases)
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- dev surface-matrix --json --output target-cynic-ci >/dev/null
	@jq -e '.status == "ok" and .summary.blocked_count == 0' target-cynic-ci/cynic-surface-audit-matrix.json >/dev/null

.PHONY: audit-runtime-phases
audit-runtime-phases: ## Emit and print machine-readable runtime phase statuses (harness/conformance/metrics)
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- dev surface-matrix --json --output target-cynic-ci >/dev/null
	@jq '.runtime_phases' target-cynic-ci/cynic-surface-audit-matrix.json

.PHONY: lab-check
lab-check: ## Validate portable lab registry and low-layer crates used by lab smoke tests
	@echo "==> checking portable lab registry"
	@jq empty $(LAB_REGISTRY)
	@echo "==> cargo check lab-facing crates"
	@$(RUST_ENV) $(CARGO) check -p cynic-host -p cynic-node -p cynic-cli

.PHONY: local-check
local-check: ## Validate single-node registry and local low-layer crates
	@echo "==> checking single-node registry"
	@jq empty $(LOCAL_REGISTRY)
	@echo "==> cargo check local-facing crates"
	@$(RUST_ENV) $(CARGO) check -p cynic-host -p cynic-node -p cynic-cli

.PHONY: lab-smoke
lab-smoke: lab-check ## Prove lab registry can drive N-case DNS and bootstrap projections without real VMs
	@tmpdir="$$(mktemp -d)"
	@echo "==> generating closed lab VM plan from $(LAB_REGISTRY)"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab plan --json --registry $(LAB_REGISTRY) > "$$tmpdir/lab-plan.json"
	@jq -e '.schema == "cynic.lab.plan.v1" and .mutates == false and .can_apply == false and .vm_count == 3 and .isolation.host_routing_mutation == false' "$$tmpdir/lab-plan.json" >/dev/null
	@echo "==> generating closed lab VM bundle from $(LAB_REGISTRY)"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab bundle --json --registry $(LAB_REGISTRY) --output-dir "$$tmpdir/lab-bundle" > "$$tmpdir/lab-bundle.json"
	@jq -e '.schema == "cynic.lab.bundle.v1" and .mutates == false and .vm_count == 3' "$$tmpdir/lab-bundle.json" >/dev/null
	@test -f "$$tmpdir/lab-bundle/manifest.json"
	@test -f "$$tmpdir/lab-bundle/network.xml"
	@echo "==> generating DNS bundle from $(LAB_REGISTRY)"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- network dns bundle --registry $(LAB_REGISTRY) --output-dir "$$tmpdir/dns" > "$$tmpdir/dns-report.json"
	@grep -R "labmesh.example" "$$tmpdir/dns" >/dev/null
	@if grep -R "synapse.cynic" "$$tmpdir/dns" >/dev/null; then echo "lab DNS bundle leaked default synapse.cynic"; exit 1; fi
	@echo "==> checking bootstrap projection from $(LAB_REGISTRY)"
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap constellation --json --registry $(LAB_REGISTRY) > "$$tmpdir/bootstrap.json" || test "$$?" = "1"
	@jq -e '.status == "bootstrap_required" and (.nodes | length == 3)' "$$tmpdir/bootstrap.json" >/dev/null
	@echo "lab smoke ok: $$tmpdir"

.PHONY: lab-plan
lab-plan: ## Emit the non-mutating closed VM lab plan
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab plan --json --registry $(LAB_REGISTRY)

.PHONY: lab-provision
lab-provision: ## Run closed VM lab provision preflight; refuses mutation until registry prerequisites are declared
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab provision --json --registry $(LAB_REGISTRY) --apply || test "$$?" = "1"

.PHONY: lab-bundle
lab-bundle: ## Generate non-mutating closed VM lab libvirt/cloud-init bundle under /tmp
	@tmpdir="$$(mktemp -d)"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab bundle --json --registry $(LAB_REGISTRY) --output-dir "$$tmpdir"

.PHONY: framework-bootstrap-bundle
framework-bootstrap-bundle: ## Generate framework bootstrap kit for a fresh Linux node
	@tmpdir="$$(mktemp -d)"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap framework bundle --json --output-dir "$$tmpdir"

# System-path CLI is the preferred default target.
CLI_USER_TARGET ?= /usr/local/bin/cynic
CLI_SYSTEM_TARGET ?= /usr/local/bin/cynic
CYNIC_CLI_ARTIFACT ?= $(CARGO_TARGET_DIR)/debug/cynic
# Rebuild cynic when any workspace Rust source or crate manifest changes.
WORKSPACE_RUST_SOURCES := $(shell find crates -name '*.rs' | tr '\n' ' ')
WORKSPACE_CARGO_MANIFESTS := $(shell find crates -name 'Cargo.toml' | tr '\n' ' ')
# Explicit source path passed to `bootstrap framework install-cli`.
# Keep this aligned with the artifact unless an operator override is required.
CYNIC_CLI_INSTALL_SOURCE ?= $(CYNIC_CLI_ARTIFACT)
CLI_USER_FALLBACK_TARGET ?= /usr/local/bin/cynic
CYNIC_CLI_AUTO_SYSTEM ?= 0

# Thin wrappers — canonical surface is `cynic deps` (human text, no make required).
.PHONY: deps
deps:
	@cynic deps 2>/dev/null || $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- deps

.PHONY: operator-day
operator-day: ## Morning loop via native cynic deps
	@cynic deps
	@echo
	@cynic deps upgrade
	@echo
	@cynic runtime inference-status --json 2>/dev/null | head -c 400 || true
	@echo
	@echo "One-click scripted upgrades:  cynic deps upgrade --yes"

.PHONY: release-deploy
release-deploy: ## Build release binaries and atomically deploy via native cynic source deploy
	@RUST_MIN_STACK=8589934592 CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" $(CARGO) build --release -p cynic-cli -p cynic-node --bin cynic --bin cynic-node --bin cynic-node-enroll
	@./target/release/cynic source deploy --apply

.PHONY: install-cli
install-cli: install-cli-auto ## Default install with preferred system-like target, with controlled fallback when unavailable

.PHONY: operator-loop
operator-loop: ## Fast operator iteration cycle: install CLI and refresh authoritative surface/kernel/runtime reports
	@echo "==> operator loop: install CLI + refresh reports"
	@$(MAKE) --no-print-directory install-cli
	@$(MAKE) --no-print-directory check-installed-runtime
	@$(MAKE) --no-print-directory audit-surface-matrix
	@$(MAKE) --no-print-directory audit-runtime-phases
	@echo "==> artifacts:"
	@echo "  - target-cynic-ci/cynic-surface-audit-matrix.json"
	@echo "  - target-cynic-ci/cynic-surface-audit-matrix.md"
	@echo "  - target-cynic-ci/cynic-ownership-audit.json"
	@if [ -f target-cynic-ci/cynic-kernel-audit.json ]; then echo "  - target-cynic-ci/cynic-kernel-audit.json"; fi

.PHONY: operator-reports
operator-reports: ## Emit operator-consumable report artifacts (surface matrix + proof bundle + metrics snapshot)
	@mkdir -p target-cynic-ci
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- dev surface-matrix --json --output target-cynic-ci >/dev/null
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime conformance --json > target-cynic-ci/cynic-runtime-conformance.json || true
	@test -s target-cynic-ci/cynic-runtime-conformance.json
	@jq -e '.proof_bundle | select(.schema == "cynic.runtime_proof_bundle.v1")' target-cynic-ci/cynic-runtime-conformance.json > target-cynic-ci/cynic-runtime-proof-bundle.json
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime metrics --json --proof target-cynic-ci/cynic-runtime-proof-bundle.json > target-cynic-ci/cynic-runtime-metrics.json
	@echo "==> operator reports:"
	@echo "  - target-cynic-ci/cynic-surface-audit-matrix.json"
	@echo "  - target-cynic-ci/cynic-surface-audit-matrix.md"
	@echo "  - target-cynic-ci/cynic-runtime-conformance.json"

.PHONY: test
test: ## Run tests for low-layer core crates
	@echo "warning: on 2026-07-16 rustc 1.96.0 can still ICE while compiling workspace tests; TEST_RUST_ENV already disables incremental compilation"
	@$(TEST_RUST_ENV) $(CARGO) test $(foreach crate,$(CORE_CRATES),-p $(crate))

.PHONY: kani-node-mesh
kani-node-mesh: ## Run Kani proofs for low-layer mesh planning invariants (requires `cargo kani setup`)
	@$(RUST_ENV) $(CARGO) kani -p cynic-node

.PHONY: kani-low-layers
kani-low-layers: ## Run aggregated low-layer Kani proofs for cynic-node (requires `cargo kani setup`)
	@$(RUST_ENV) $(CARGO) kani -p cynic-node --lib --exact \
		--harness dns_contract::verification::proof_default_dns_zone_constant_is_stable \
		--harness dns_contract::verification::proof_dns_bundle_files_shape_is_stable \
		--harness dns_contract::verification::proof_dns_policy_surface_tracks_requested_zone \
		--harness dns_contract::verification::proof_dns_required_modes_are_uniform_0644 \
		--harness dns_contract::verification::proof_dns_source_lists_keep_canonical_guardrails \
		--harness network_mesh_plan::verification::proof_address_plane_prefers_declared_service_plane \
		--harness network_mesh_plan::verification::proof_address_plane_falls_back_to_canonical_plane \
		--harness network_mesh_plan::verification::proof_address_plane_defaults_to_synapse_without_match \
		--harness network_mesh_plan::verification::proof_default_dns_names_follow_zone_presence \
		--harness network_mesh_apply_plan::command_plan_verification::proof_route_install_uses_injected_ip_bin_and_mesh_interface \
		--harness network_mesh_apply_plan::command_plan_verification::proof_post_apply_observation_adds_dns_probe_only_when_dns_exists \
		--harness network_mesh_apply_plan::command_plan_verification::proof_firewall_preflight_reports_acl_cardinality_and_cidr \
		--harness network_mesh_apply_plan::command_plan_verification::proof_rollback_plan_includes_route_removal_and_tail_expectations \
		--harness network_mesh_apply_plan::command_plan_verification::proof_dns_install_is_bundle_relative \
		--harness network_mesh_apply_plan::policy_verification::proof_mesh_policy_exposes_stable_profile_order_and_count \
		--harness network_mesh_apply_plan::policy_verification::proof_mesh_policy_forbidden_tokens_keep_shell_escape_vectors_blocked \
		--harness network_mesh_apply_plan::policy_verification::proof_mesh_policy_preserves_argv_surfaces_verbatim \
		--harness network_root_policy::verification::proof_root_policy_keeps_denial_contract_shape \
		--harness network_root_policy::verification::proof_remediation_to_profile_maps_only_root_network_actions \
		--harness network_root_policy::verification::proof_dns_remediation_never_becomes_root_profile \
		--harness network_root_policy::verification::proof_allowed_profiles_only_contains_root_remediations

.PHONY: run-kernel
run-kernel: ## Run the kernel locally
	@$(RUST_ENV) $(CARGO) run -p cynic-kernel

.PHONY: run-node
run-node: ## Run the node locally
	@$(RUST_ENV) $(CARGO) run -p cynic-node

.PHONY: install-user-bins
install-user-bins: build ## Install release binaries into ~/bin
	@mkdir -p "$$HOME/bin"
	@install -m 0755 "$(CARGO_TARGET_DIR)/debug/cynic-kernel" "$$HOME/bin/cynic-kernel"
	@install -m 0755 "$(CARGO_TARGET_DIR)/debug/cynic-node" "$$HOME/bin/cynic-node"
	@install -m 0755 "$(CARGO_TARGET_DIR)/debug/cynic" "$$HOME/bin/cynic"

.PHONY: systemd-list
systemd-list: ## List remaining runtime unit templates
	@find infra/systemd -maxdepth 1 -type f | sort
