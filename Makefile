SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.NOTPARALLEL:

CARGO ?= cargo
CARGO_NETWORK_MODE ?= offline
CARGO_SOURCE_MODE ?= registry
CYNIC_CARGO_TARGET_ROOT ?= /tmp/cynic-cargo-targets
CYNIC_CARGO_TARGET_PROFILE ?= default
CYNIC_CARGO_BUILD_JOBS ?= 1
CARGO_TARGET_DIR := $(if $(strip $(CARGO_TARGET_DIR)),$(CARGO_TARGET_DIR),$(CYNIC_CARGO_TARGET_ROOT)/$(CYNIC_CARGO_TARGET_PROFILE))

# One crate list. It previously existed twice — CORE_CRATES (7) and
# LAYER_CRATES (5) — differing by exactly the two crates that hold the most
# uncompiled mass, so `check.layers` silently skipped cynic-mcp and
# cynic-inference while `make test` covered them. Two lists in one name space
# is how a crate stops being tested without anyone deciding it should.
CORE_CRATES := cynic-protocol cynic-host cynic-node cynic-kernel cynic-cli cynic-inference cynic-mcp

LAB_REGISTRY := infra/registries/lab.json
LOCAL_REGISTRY := infra/registries/single-node.json

CYNIC_BUILD_ADMISSION_GRANT ?=
CYNIC_BUILD_WORKLOAD_RECORD ?=
CYNIC_BUILD_AUTHORITY_PUBLIC_KEY ?=
CYNIC_BUILD_ADMISSION_VERIFIER ?= cynic-node
CYNIC_BUILD_BOOTSTRAP_RECEIPT ?=

# Bounded compiler stack. 256 MiB is the proven default on <=~32 GiB runners.
# The previous hardcodes of 8G/16G/32G escalated memory, which produced MORE
# cold rebuilds and therefore MORE transitive dependency ICE exposure
# (ring/winnow/regex-automata). The toolchain is pinned per crate by
# rust-toolchain.toml, so no RUSTUP_TOOLCHAIN is hardcoded here.
CYNIC_RUST_MIN_STACK ?= 268435456

RUST_ENV := CYNIC_BUILD_ADMISSION_GRANT=$(CYNIC_BUILD_ADMISSION_GRANT) CYNIC_BUILD_WORKLOAD_RECORD=$(CYNIC_BUILD_WORKLOAD_RECORD) CYNIC_BUILD_AUTHORITY_PUBLIC_KEY=$(CYNIC_BUILD_AUTHORITY_PUBLIC_KEY) CYNIC_BUILD_ADMISSION_VERIFIER=$(CYNIC_BUILD_ADMISSION_VERIFIER) CYNIC_BUILD_BOOTSTRAP_RECEIPT=$(CYNIC_BUILD_BOOTSTRAP_RECEIPT) RUST_MIN_STACK=$(CYNIC_RUST_MIN_STACK) CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=$(CYNIC_CARGO_BUILD_JOBS) CMAKE_BUILD_PARALLEL_LEVEL=$(CYNIC_CARGO_BUILD_JOBS) CARGO_TARGET_DIR=$(CARGO_TARGET_DIR) CYNIC_CARGO_NETWORK_MODE=$(CYNIC_CARGO_NETWORK_MODE) CYNIC_CARGO_SOURCE_MODE=$(CYNIC_CARGO_SOURCE_MODE)

# Test builds monomorphise far harder than check builds: cynic-node's test
# harness is memory-confirmed at 8 GiB. `check.layers` used to run its
# `cargo test` under the 256 MiB RUST_ENV while the comment right above it
# said tests need 8 GiB, so the one target that both checked and tested was
# the one configured for the smaller stack.
TEST_RUST_ENV := $(RUST_ENV) RUST_MIN_STACK=8589934592

# Release builds go through RUST_ENV like everything else. `release.deploy` used
# to be the one target that bypassed it, so the build that produces the
# artifact an operator deploys was the one that silently ignored the admission
# grant, the network mode and the target directory — then invoked
# ./target/release/cynic, a path that only exists because CARGO_TARGET_DIR was
# not actually being passed.
RELEASE_ENV := $(RUST_ENV) RUST_MIN_STACK=8589934592 RUSTFLAGS='-C debuginfo=0'

# A `cynic` on PATH may be stale, unmanaged, or absent. Resolve once, prefer a
# managed install, and never fall back to a path that only exists on a build
# runner. The operator host once resolved `cynic` to an unmanaged 0.2.0 debug
# build that served four CLI surfaces HEAD cannot produce, which made
# `make check` validate an artifact the source no longer described.
CYNIC ?= $(shell command -v cynic 2>/dev/null || true)
CYNIC_BIN := $(if $(strip $(CYNIC)),$(CYNIC),$(CARGO_TARGET_DIR)/debug/cynic)

# Fail before spending a build when a target's CLI surface cannot be produced
# from this tree. 13 of 31 targets used to call a surface that does not exist
# in HEAD — `dev`, `lab`, `bootstrap`, `deps`, `secret` — and each one burned a
# workspace build before printing "unrecognized subcommand", which reads like a
# build failure rather than a routing gap. A target that cannot work must say
# so in one line.
define require_surface
	@missing=""; \
	for sub in $(1); do \
		if ! $(CYNIC_BIN) $$sub --help >/dev/null 2>&1; then missing="$$missing $$sub"; fi; \
	done; \
	if [ -n "$$missing" ]; then \
		echo "make $(@): not runnable — CLI surface(s) not routable from this tree:$$missing" >&2; \
		echo "  These are implemented and dispatched but not reachable from argv; see" >&2; \
		echo "  infra/CLI_SURFACE_AUDIT.md and infra/WORKSPACE_ORPHAN_DECISIONS.md." >&2; \
		exit 1; \
	fi
endef

.PHONY: help
help: ## List targets
	@grep -hE '^[a-zA-Z][a-zA-Z0-9_.-]*:.*?## ' $(MAKEFILE_LIST) \
		| sed 's/:.*## /\t/' | sort | awk -F'\t' '{printf "  %-28s %s\n", $$1, $$2}'

.PHONY: secret.literals
secret.literals: ## Reject high-confidence literal credentials in Git-tracked source
	$(call require_surface,secret)
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- secret lint

.PHONY: authority.check
authority.check: secret.literals ## Read-only local validation of machine-readable authorities
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
	$(call require_surface,dev)
	@CYNIC_ALLOW_LOCAL_DEV_BUILD=1 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- dev surface-registry --check
	@for script in $$(git grep -l '^#!' -- scripts); do \
		mode="$$(git ls-files -s -- "$$script" | awk '{print $$1}')"
		if [[ "$$mode" != "100755" ]]; then
			echo "tracked script is not executable in Git: $$script ($${mode:-untracked})" >&2
			exit 1
		fi
	done

.PHONY: check
check: ## Native validation via cynic audit crate (no make recursion, exit code is the gate)
	@echo "==> cynic audit crate (native, 100% Rust)"
	@if [ -x "$(CYNIC_BIN)" ]; then \
		$(CYNIC_BIN) audit crate --json; \
	else \
		$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- audit crate --json; \
	fi

.PHONY: audit
audit: ## Native unified audit of all crates, architecture boundaries, and secrets
	@if [ -x "$(CYNIC_BIN)" ]; then \
		$(CYNIC_BIN) audit all --json; \
	else \
		$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- audit all --json; \
	fi

.PHONY: converge
converge: ## Native automated convergence of worktrees, branches, and unified audit
	@echo "==> Reconciling agent worktrees and branches"
	@if [ -x "$(CYNIC_BIN)" ]; then \
		$(CYNIC_BIN) source worktree --apply --json; \
		echo "==> Verifying workspace convergence audit"; \
		$(CYNIC_BIN) audit all --json; \
	else \
		$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- source worktree --apply --json; \
		echo "==> Verifying workspace convergence audit"; \
		$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- audit all --json; \
	fi

# Per-layer check+test, fail-closed, dependency-ordered, over the single crate
# list. Cache integrity is observed first so a stale vendored cache is
# reported rather than papered over.
.PHONY: check.layers
check.layers: ## Per-layer cargo check+test, fail-closed, dependency-ordered
	@echo "==> cache integrity (offline/locked metadata)"
	@$(CARGO) metadata --offline --locked --format-version 1 >/dev/null \
		|| { echo "cache integrity: offline/locked metadata unavailable (offline_dependency_unavailable)"; exit 1; }
	@for crate in $(CORE_CRATES); do \
		echo "  -> $$crate"; \
		( cd "crates/$$crate" && $(RUST_ENV) $(CARGO) check -p "$$crate" ) || exit 1; \
		( cd "crates/$$crate" && $(TEST_RUST_ENV) $(CARGO) test -p "$$crate" ) || exit 1; \
	done
	@echo "==> check.layers: all layers validated"

.PHONY: check.ratchets
check.ratchets: ## Run the shrink-only architecture check.ratchets (orphans, god objects, legacy debt)
	@echo "==> architecture check.ratchets (shrink-only, fail in both directions)"
	@for crate in $(CORE_CRATES); do \
		if [ -f "crates/$$crate/tests/orphan_sources_ratchet.rs" ]; then \
			echo "  -> $$crate orphan sources"; \
			$(TEST_RUST_ENV) $(CARGO) test -q -p "$$crate" --test orphan_sources_ratchet || exit 1; \
		fi; \
	done
	@echo "  -> crate size ceilings and legacy debt"
	@$(TEST_RUST_ENV) $(CARGO) test -q -p cynic-cli --test crate_loc_ratchet

.PHONY: check.installed
check.installed: ## Validate installed-runtime portability invariants
	@$(RUST_ENV) $(CARGO) test -q -p cynic-cli --test installed_runtime

.PHONY: check.strict
check.strict: ## cargo check core crates with -D warnings (signal engineering debt)
	@echo "==> strict warning check on core crates"
	@$(RUST_ENV) RUSTFLAGS='-D warnings' $(CARGO) check $(foreach crate,$(CORE_CRATES),-p $(crate))

.PHONY: audit.surfaces
audit.surfaces: ## Verify canonical CLI surfaces are backed by real modules and mutation policies
	$(call require_surface,dev)
	@echo "==> auditing canonical CLI surfaces"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- dev audit.surfaces --json | jq -e '.schema == "cynic.cli.surface_audit.v1" and .status == "ok" and .blocked_count == 0' >/dev/null

.PHONY: audit.matrix
audit.matrix: ## Emit reusable JSON+Markdown surface matrix (surface, ownership, kernel-audit, runtime phases)
	$(call require_surface,dev)
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- dev surface-matrix --json --output target-cynic-ci >/dev/null
	@jq -e '.status == "ok" and .summary.blocked_count == 0' target-cynic-ci/cynic-surface-audit-matrix.json >/dev/null

.PHONY: audit.phases
audit.phases: ## Emit and print machine-readable runtime phase statuses
	$(call require_surface,dev)
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- dev surface-matrix --json --output target-cynic-ci >/dev/null
	@jq '.runtime_phases' target-cynic-ci/cynic-surface-audit-matrix.json

.PHONY: lab.check
lab.check: ## Validate portable lab registry and low-layer crates used by lab smoke tests
	@echo "==> checking portable lab registry"
	@jq empty $(LAB_REGISTRY)
	@echo "==> cargo check lab-facing crates"
	@$(RUST_ENV) $(CARGO) check -p cynic-host -p cynic-node -p cynic-cli

.PHONY: local.check
local.check: ## Validate single-node registry and local low-layer crates
	@echo "==> checking single-node registry"
	@jq empty $(LOCAL_REGISTRY)
	@echo "==> cargo check local-facing crates"
	@$(RUST_ENV) $(CARGO) check -p cynic-host -p cynic-node -p cynic-cli

.PHONY: lab.smoke
lab.smoke: lab.check ## Prove the lab registry drives N-case DNS and bootstrap projections without real VMs
	$(call require_surface,lab bootstrap network)
	@tmpdir="$$(mktemp -d)"
	@echo "==> generating closed lab VM plan from $(LAB_REGISTRY)"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab plan --json --registry $(LAB_REGISTRY) > "$$tmpdir/lab.plan.json"
	@jq -e '.schema == "cynic.lab.plan.v1" and .mutates == false and .can_apply == false and .vm_count == 3 and .isolation.host_routing_mutation == false' "$$tmpdir/lab.plan.json" >/dev/null
	@echo "==> generating closed lab VM bundle from $(LAB_REGISTRY)"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab bundle --json --registry $(LAB_REGISTRY) --output-dir "$$tmpdir/lab.bundle" > "$$tmpdir/lab.bundle.json"
	@jq -e '.schema == "cynic.lab.bundle.v1" and .mutates == false and .vm_count == 3' "$$tmpdir/lab.bundle.json" >/dev/null
	@test -f "$$tmpdir/lab.bundle/manifest.json"
	@test -f "$$tmpdir/lab.bundle/network.xml"
	@echo "==> generating DNS bundle from $(LAB_REGISTRY)"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- network dns bundle --registry $(LAB_REGISTRY) --output-dir "$$tmpdir/dns" > "$$tmpdir/dns-report.json"
	@grep -R "labmesh.example" "$$tmpdir/dns" >/dev/null
	@if grep -R "synapse.cynic" "$$tmpdir/dns" >/dev/null; then echo "lab DNS bundle leaked default synapse.cynic"; exit 1; fi
	@echo "==> checking bootstrap projection from $(LAB_REGISTRY)"
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap constellation --json --registry $(LAB_REGISTRY) > "$$tmpdir/bootstrap.json" || test "$$?" = "1"
	@jq -e '.status == "bootstrap_required" and (.nodes | length == 3)' "$$tmpdir/bootstrap.json" >/dev/null
	@echo "lab smoke ok: $$tmpdir"

.PHONY: lab.plan
lab.plan: ## Emit the non-mutating closed VM lab plan
	$(call require_surface,lab)
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab plan --json --registry $(LAB_REGISTRY)

.PHONY: lab.provision
lab.provision: ## Run closed VM lab provision preflight; refuses mutation until prerequisites are declared
	$(call require_surface,lab)
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab provision --json --registry $(LAB_REGISTRY) --apply || test "$$?" = "1"

.PHONY: lab.bundle
lab.bundle: ## Generate non-mutating closed VM lab libvirt/cloud-init bundle under /tmp
	$(call require_surface,lab)
	@tmpdir="$$(mktemp -d)"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab bundle --json --registry $(LAB_REGISTRY) --output-dir "$$tmpdir"

.PHONY: framework.bootstrap
framework.bootstrap: ## Generate framework bootstrap kit for a fresh Linux node
	$(call require_surface,bootstrap)
	@tmpdir="$$(mktemp -d)"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap framework bundle --json --output-dir "$$tmpdir"

# Managed install target. The previous `install.cli` was a target with no
# recipe whose only prerequisite, `install-cli-auto`, was defined nowhere, so
# `make install.cli` and `make operator.loop` both died on a make error.
CLI_USER_TARGET ?= /usr/local/bin/cynic
CYNIC_CLI_ARTIFACT ?= $(CARGO_TARGET_DIR)/debug/cynic

.PHONY: install.cli
install.cli: ## Build the CLI and install it to CLI_USER_TARGET (default /usr/local/bin/cynic)
	@echo "==> building cynic"
	@$(RUST_ENV) $(CARGO) build -p cynic-cli --bin cynic
	@echo "==> installing to $(CLI_USER_TARGET)"
	@install -m 0755 "$(CYNIC_CLI_ARTIFACT)" "$(CLI_USER_TARGET)"
	@echo "==> read-back"
	@"$(CLI_USER_TARGET)" version

.PHONY: deps
deps: ## Dependency census (requires the `deps` CLI surface)
	$(call require_surface,deps)
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- deps

.PHONY: operator.day
operator.day: ## Morning loop via native cynic deps
	$(call require_surface,deps runtime)
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- deps
	@echo
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- deps upgrade
	@echo
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime inference-status --json 2>/dev/null | head -c 400 || true
	@echo
	@echo "One-click scripted upgrades:  cynic deps upgrade --yes"

.PHONY: release.deploy
release.deploy: ## Build release binaries and atomically deploy via native cynic source deploy
	@$(RELEASE_ENV) $(CARGO) build --release -p cynic-cli -p cynic-node --bin cynic --bin cynic-node --bin cynic-node-enroll
	@"$(CARGO_TARGET_DIR)/release/cynic" source deploy --apply

.PHONY: operator.loop
operator.loop: ## Fast operator iteration: install CLI and refresh authoritative reports
	@echo "==> operator loop: install CLI + refresh reports"
	@$(MAKE) --no-print-directory install.cli
	@$(MAKE) --no-print-directory check.installed
	@$(MAKE) --no-print-directory audit.matrix
	@$(MAKE) --no-print-directory audit.phases
	@echo "==> artifacts:"
	@echo "  - target-cynic-ci/cynic-surface-audit-matrix.json"
	@echo "  - target-cynic-ci/cynic-surface-audit-matrix.md"
	@if [ -f target-cynic-ci/cynic-ownership-audit.json ]; then echo "  - target-cynic-ci/cynic-ownership-audit.json"; fi
	@if [ -f target-cynic-ci/cynic-kernel-audit.json ]; then echo "  - target-cynic-ci/cynic-kernel-audit.json"; fi

.PHONY: operator.reports
operator.reports: ## Emit operator-consumable report artifacts (surface matrix + proof bundle + metrics)
	$(call require_surface,dev runtime)
	@mkdir -p target-cynic-ci
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- dev surface-matrix --json --output target-cynic-ci >/dev/null
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime conformance --json > target-cynic-ci/cynic-runtime-conformance.json || true
	@test -s target-cynic-ci/cynic-runtime-conformance.json
	@jq -e '.proof_bundle | select(.schema == "cynic.runtime_proof_bundle.v1")' target-cynic-ci/cynic-runtime-conformance.json > target-cynic-ci/cynic-runtime-proof-bundle.json
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime metrics --json --proof target-cynic-ci/cynic-runtime-proof-bundle.json > target-cynic-ci/cynic-runtime-metrics.json
	@echo "==> operator reports:"
	@echo "  - target-cynic-ci/cynic-surface-audit-matrix.json"
	@echo "  - target-cynic-ci/cynic-runtime-conformance.json"

.PHONY: test
test: ## Run tests for the core crates
	@echo "note: rustc can ICE on workspace test builds; TEST_RUST_ENV raises the stack and disables incremental"
	@$(TEST_RUST_ENV) $(CARGO) test $(foreach crate,$(CORE_CRATES),-p $(crate))

.PHONY: kani.mesh
kani.mesh: ## Run Kani proofs for low-layer mesh planning invariants (requires `cargo kani setup`)
	@$(RUST_ENV) $(CARGO) kani -p cynic-node

.PHONY: kani.layers
kani.layers: ## Run aggregated low-layer Kani proofs for cynic-node (requires `cargo kani setup`)
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

.PHONY: run.kernel
run.kernel: ## Run the kernel locally
	@$(RUST_ENV) $(CARGO) run -p cynic-kernel

.PHONY: run.node
run.node: ## Run the node locally
	@$(RUST_ENV) $(CARGO) run -p cynic-node

.PHONY: install.bins
install.bins: ## Install built binaries into ~/bin
	@mkdir -p "$$HOME/bin"
	@for bin in cynic cynic-node cynic-kernel; do \
		src="$(CARGO_TARGET_DIR)/debug/$$bin"; \
		if [ -x "$$src" ]; then \
			install -m 0755 "$$src" "$$HOME/bin/$$bin"; \
			echo "  installed $$HOME/bin/$$bin"; \
		else \
			echo "  skipped $$bin (not built at $$src)" >&2; \
		fi; \
	done

.PHONY: systemd.list
systemd.list: ## List remaining runtime unit templates
	@find infra/systemd -maxdepth 1 -type f | sort
