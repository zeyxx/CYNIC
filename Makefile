SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.NOTPARALLEL:

PROJECT_DIR := $(shell git rev-parse --show-toplevel 2>/dev/null || pwd)
PATH := $(PROJECT_DIR)/scripts:$(PATH)
CARGO := $(PROJECT_DIR)/scripts/cargo
CARGO_NETWORK_MODE ?= offline
CARGO_SOURCE_MODE ?= registry
CYNIC_CARGO_TARGET_ROOT ?= /tmp/cynic-cargo-targets
CYNIC_CARGO_TARGET_PROFILE ?= default
CYNIC_CARGO_BUILD_JOBS ?= 1
CARGO_TARGET_DIR := $(if $(strip $(CARGO_TARGET_DIR)),$(CARGO_TARGET_DIR),$(CYNIC_CARGO_TARGET_ROOT)/$(CYNIC_CARGO_TARGET_PROFILE))
CORE_CRATES := cynic-protocol cynic-host cynic-node cynic-kernel cynic-mcp cynic-cli cynic-askesis
LAB_REGISTRY := infra/registries/lab.json
LOCAL_REGISTRY := infra/registries/single-node.json
CYNIC_BUILD_ADMISSION_GRANT ?=
CYNIC_BUILD_WORKLOAD_RECORD ?=
CYNIC_BUILD_AUTHORITY_PUBLIC_KEY ?=
CYNIC_BUILD_ADMISSION_VERIFIER ?= cynic-node
CYNIC_BUILD_BOOTSTRAP_RECEIPT ?=
# Keep a bounded large compiler stack for kernel/CLI monomorphization. The
# repository toolchain is pinned because rustc 1.96 exhibited ICE/SIGSEGV here.
RUST_ENV := CYNIC_BUILD_ADMISSION_GRANT=$(CYNIC_BUILD_ADMISSION_GRANT) CYNIC_BUILD_WORKLOAD_RECORD=$(CYNIC_BUILD_WORKLOAD_RECORD) CYNIC_BUILD_AUTHORITY_PUBLIC_KEY=$(CYNIC_BUILD_AUTHORITY_PUBLIC_KEY) CYNIC_BUILD_ADMISSION_VERIFIER=$(CYNIC_BUILD_ADMISSION_VERIFIER) CYNIC_BUILD_BOOTSTRAP_RECEIPT=$(CYNIC_BUILD_BOOTSTRAP_RECEIPT) RUST_MIN_STACK=2147483648 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=$(CYNIC_CARGO_BUILD_JOBS) CMAKE_BUILD_PARALLEL_LEVEL=$(CYNIC_CARGO_BUILD_JOBS) CARGO_TARGET_DIR=$(CARGO_TARGET_DIR) CYNIC_CARGO_NETWORK_MODE=$(CARGO_NETWORK_MODE) CYNIC_CARGO_SOURCE_MODE=$(CARGO_SOURCE_MODE)
BUILD_RUST_ENV := $(RUST_ENV)
RELEASE_RUST_ENV := $(RUST_ENV)
TEST_RUST_ENV := $(RUST_ENV)

.PHONY: authority-check
authority-check: secret-literal-check ## Read-only local validation of machine-readable authorities
	@echo "==> checking foundation, runtime authority, and registry"
	@jq empty contracts/foundation.json
	@jq empty infra/runtime-authority.json
	@jq empty infra/registry.json
	@jq empty infra/agent-exposure.json
	@jq empty infra/ecosystem-pins.json
	@jq empty infra/runtime-basket.json
	@jq -e '.schema == "cynic.surface_registry.v1" and (.surfaces | type == "array" and length > 0) and (([.surfaces[].id] | length) == ([.surfaces[].id] | unique | length))' infra/surface-registry/manifest.json >/dev/null
	@while IFS=$$'\t' read -r surface_id fragment_path; do
		if [[ "$$fragment_path" == /* || "$$fragment_path" == *..* || "$$fragment_path" == *\\* ]]; then
			echo "unsafe surface registry fragment path: $$fragment_path" >&2
			exit 1
		fi
		jq -e --arg id "$$surface_id" '.id == $$id' "infra/surface-registry/$$fragment_path" >/dev/null
	done < <(jq -r '.surfaces[] | [.id, .path] | @tsv' infra/surface-registry/manifest.json)
	@for script in $$(git grep -l '^#!' -- scripts); do
		mode="$$(git ls-files -s -- "$$script" | awk '{print $$1}')"
		if [[ "$$mode" != "100755" ]]; then
			echo "tracked script is not executable in Git: $$script ($${mode:-untracked})" >&2
			exit 1
		fi
	done

.PHONY: secret-literal-check
secret-literal-check: ## Reject high-confidence literal credentials in Git-tracked source
	@./scripts/secret-literal-check

.PHONY: check
check: authority-check build-admission-check ## Build-runner validation of the living low-layer Rust runtime baseline
	@echo "==> cargo check core crates"
	@$(RUST_ENV) $(CARGO) check $(foreach crate,$(CORE_CRATES),-p $(crate))
	@$(MAKE) --no-print-directory surface-audit
	@$(RUST_ENV) $(CARGO) test -q -p cynic-cli --test installed_runtime

.PHONY: build-admission-check
build-admission-check:
	@CYNIC_BUILD_ADMISSION_GRANT="$(CYNIC_BUILD_ADMISSION_GRANT)" \
		CYNIC_BUILD_WORKLOAD_RECORD="$(CYNIC_BUILD_WORKLOAD_RECORD)" \
		CYNIC_BUILD_AUTHORITY_PUBLIC_KEY="$(CYNIC_BUILD_AUTHORITY_PUBLIC_KEY)" \
		CYNIC_BUILD_ADMISSION_VERIFIER="$(CYNIC_BUILD_ADMISSION_VERIFIER)" \
		CYNIC_BUILD_BOOTSTRAP_RECEIPT="$(CYNIC_BUILD_BOOTSTRAP_RECEIPT)" \
		./scripts/build-admission-check make:check

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
	@echo "  - target-cynic-ci/cynic-runtime-proof-bundle.json"
	@echo "  - target-cynic-ci/cynic-runtime-metrics.json"
	@jq -r '"  status=\(.status)\n  maturity_percent=\(.maturity_percent)\n  residual_unknown_percent=\(.residual_unknown_percent)"' \
		target-cynic-ci/cynic-runtime-metrics.json

.PHONY: install-cli-auto
install-cli-auto:
	@if [ -w "$$(dirname "$(CLI_USER_TARGET)")" ] && [ -x "$$(dirname "$(CLI_USER_TARGET)")" ]; then \
		$(MAKE) --no-print-directory install-cli-user CLI_USER_TARGET="$(CLI_USER_TARGET)"; \
	else \
		echo "==> user cli target not writable: $$(dirname "$(CLI_USER_TARGET)")"; \
		if mkdir -p "$$(dirname "$(CLI_USER_FALLBACK_TARGET)")" 2>/dev/null \
			&& [ -w "$$(dirname "$(CLI_USER_FALLBACK_TARGET)")" ] \
			&& [ -x "$$(dirname "$(CLI_USER_FALLBACK_TARGET)")" ]; then \
			echo "==> using writable fallback user target: $(CLI_USER_FALLBACK_TARGET)"; \
			$(MAKE) --no-print-directory install-cli-user CLI_USER_TARGET="$(CLI_USER_FALLBACK_TARGET)"; \
		elif [ "$(CYNIC_CLI_AUTO_SYSTEM)" = "1" ]; then \
			echo "==> falling back to system install surface (requires sudo)"; \
			$(MAKE) --no-print-directory install-cli-system; \
		else \
			echo "==> blocked: set CLI_USER_TARGET to a writable path, set CLI_USER_FALLBACK_TARGET, or run CYNIC_CLI_AUTO_SYSTEM=1 make install-cli"; \
			exit 1; \
		fi; \
	fi

.PHONY: install-cli-user
$(CLI_USER_TARGET): $(CYNIC_CLI_INSTALL_SOURCE)
	@if [ ! -x "$(CYNIC_CLI_INSTALL_SOURCE)" ]; then \
		echo "==> blocked: source binary is not executable: $(CYNIC_CLI_INSTALL_SOURCE)"; \
		echo "    set CYNIC_CLI_INSTALL_SOURCE explicitly to a valid cynic artifact, or rebuild default target"; \
		exit 1; \
	fi
	@if ! mkdir -p "$$(dirname "$(CLI_USER_TARGET)")" 2>/dev/null; then \
		echo "==> blocked: cannot create user target directory: $$(dirname "$(CLI_USER_TARGET)")"; \
		echo "    set CLI_USER_TARGET to a writable path, or run install-cli-system"; \
		exit 1; \
	fi
	@if [ -f "$(CLI_USER_TARGET)" ] && cmp -s "$(CLI_USER_TARGET)" "$(CYNIC_CLI_INSTALL_SOURCE)"; then \
		echo "==> user target already matches source ($(CLI_USER_TARGET)); skipping install"; \
		exit 0; \
	fi
	@if [ ! -w "$$(dirname "$(CLI_USER_TARGET)")" ]; then \
		echo "==> blocked: user install target directory is not writable: $$(dirname "$(CLI_USER_TARGET)")"; \
		echo "    set CLI_USER_TARGET to a writable path, or run install-cli-system"; \
		exit 1; \
	fi
	@echo "==> installing to $(CLI_USER_TARGET)"
	@"$(CYNIC_CLI_INSTALL_SOURCE)" bootstrap framework install-cli --json --apply \
		--source "$(CYNIC_CLI_INSTALL_SOURCE)" \
		--target "$(CLI_USER_TARGET)"

install-cli-user: $(CLI_USER_TARGET) ## Build and install cynic to the chosen target path (no sudo; preferred for dev)
	@echo "==> which installed cynic target => $(CLI_USER_TARGET)"
	@command -v "$(CLI_USER_TARGET)" >/dev/null && "$(CLI_USER_TARGET)" runtime --help 2>/dev/null | grep -q inference-start \
		&& echo "==> ok: inference-start present" \
		|| echo "==> warn: runtime entrypoint probe unavailable (PATH may still point to older cynic)"
	@if [ "$$(dirname "$(CLI_USER_TARGET)")" = "/tmp/cynic/bin" ]; then \
		echo "==> if needed, expose this binary with: export PATH=$$PATH:/tmp/cynic/bin"; \
	fi

.PHONY: install-cli-system-refresh
install-cli-system-refresh: ## Rebuild from artifact and force a CLI refresh with explicit source declaration
	@$(MAKE) --no-print-directory install-cli-system \
		CYNIC_CLI_INSTALL_SOURCE="$(CYNIC_CLI_ARTIFACT)"

.PHONY: install-cli-system
install-cli-system: $(CYNIC_CLI_INSTALL_SOURCE)
	@if [ ! -x "$(CYNIC_CLI_INSTALL_SOURCE)" ]; then \
		echo "==> blocked: source binary is not executable: $(CYNIC_CLI_INSTALL_SOURCE)"; \
		echo "    set CYNIC_CLI_INSTALL_SOURCE explicitly to a valid cynic artifact, or rebuild default target"; \
		exit 1; \
	fi
	@if [ -f "$(CLI_SYSTEM_TARGET)" ] && cmp -s "$(CLI_SYSTEM_TARGET)" "$(CYNIC_CLI_INSTALL_SOURCE)"; then \
		echo "==> system target already matches source ($(CLI_SYSTEM_TARGET)); skipping install"; \
		exit 0; \
	fi
	@echo "==> installing to $(CLI_SYSTEM_TARGET) (root)"
	@sudo "$(CYNIC_CLI_INSTALL_SOURCE)" bootstrap framework install-cli --json --apply \
		--source "$(CYNIC_CLI_INSTALL_SOURCE)" \
		--target "$(CLI_SYSTEM_TARGET)"
	@if [ ! -x "$(CLI_SYSTEM_TARGET)" ]; then echo "==> warning: installed file not yet executable"; fi
	@echo "==> which cynic => $$(command -v cynic)"

$(CYNIC_CLI_ARTIFACT): $(WORKSPACE_RUST_SOURCES) $(WORKSPACE_CARGO_MANIFESTS) Cargo.toml Cargo.lock
	@echo "==> building cynic-cli"
	@$(BUILD_RUST_ENV) $(CARGO) build -p cynic-cli --bin cynic

.PHONY: lab-guest-bootstrap-bundle
lab-guest-bootstrap-bundle: framework-bootstrap-bundle ## Compatibility alias for the closed lab bootstrap usecase

.PHONY: lab-image-plan
lab-image-plan: ## Emit base image download/checksum preflight without downloading
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab image plan --json --registry $(LAB_REGISTRY)

.PHONY: lab-image-resolve
lab-image-resolve: ## Resolve expected base image sha256 from upstream SHA256SUMS without editing registry
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab image resolve --json --registry $(LAB_REGISTRY)

.PHONY: lab-image-fetch
lab-image-fetch: ## Refuse image fetch until registry has a pinned sha256
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- lab image fetch --json --registry $(LAB_REGISTRY) --apply || test "$$?" = "1"

.PHONY: runtime-validate
runtime-validate: ## Execute read-only CLI runtime validation against fleet + portable lab registries
	@$(MAKE) --no-print-directory lab-check >&2
	@echo "==> executing runtime validation" >&2
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime validate --json --registry infra/registry.json --lab-registry $(LAB_REGISTRY)

.PHONY: runtime-suite
runtime-suite: ## Emit the stable IaC execution suite for the portable lab registry
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime suite --json --registry $(LAB_REGISTRY) --profile lab

.PHONY: runtime-plan
runtime-plan: ## Emit the current non-mutating execution plan for the portable lab registry
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime plan --json --registry $(LAB_REGISTRY) --profile lab

.PHONY: runtime-harness
runtime-harness: ## Emit the canonical runtime harness progression and execution membranes
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime harness --json

.PHONY: runtime-conformance
runtime-conformance: ## Observe host conformance for this node without mutating host/network state
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime conformance --json --registry $(LOCAL_REGISTRY) || test "$$?" = "1"

.PHONY: runtime-metrics
runtime-metrics: ## Emit percent-based runtime metrics; unknown metrics remain null
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime metrics --json --registry $(LOCAL_REGISTRY)

.PHONY: runtime-metrics-architecture
runtime-metrics-architecture: ## Run architecture boundary tests and consume their proof in runtime metrics
	@tmpdir="$$(mktemp -d)"
	@$(TEST_RUST_ENV) $(CARGO) test -q -p cynic-cli --test architecture_boundary >&2
	@jq -n '{schema:"cynic.runtime_proof_bundle.v1",status:"satisfied",proofs:[{schema:"cynic.runtime_proof.v1",gate:"architecture_boundary_verified",status:"satisfied",producer:"cargo-test",subject:"cynic-cli/tests/architecture_boundary.rs",observed_at_unix_ms:1,evidence:[{id:"architecture_boundary",status:"active",detail:"cargo test -p cynic-cli --test architecture_boundary passed"}]}]}' > "$$tmpdir/architecture-proof.json"
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime metrics --json --registry $(LOCAL_REGISTRY) --proof "$$tmpdir/architecture-proof.json"

.PHONY: local-plan
local-plan: ## Emit the current non-mutating execution plan for this single machine
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime plan --json --registry $(LOCAL_REGISTRY) --profile single-node

.PHONY: local-host-proof
local-host-proof: ## Observe this host and emit host_identity_observed proof bundle
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap host proof --node local-kernel --registry $(LOCAL_REGISTRY) || test "$$?" = "1"

.PHONY: local-host-plan
local-host-plan: ## Emit the local host bootstrap plan without mutating this machine
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap host plan --node local-kernel --registry $(LOCAL_REGISTRY)

.PHONY: host-bridge-bundle
host-bridge-bundle: ## Build the bounded host-exec bridge bundle under /tmp
	@$(RUST_ENV) $(CARGO) build -q -p cynic-host --bin cynic-host-exec
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap bridge bundle --output-dir /tmp/cynic-host-exec

.PHONY: host-bridge-plan
host-bridge-plan: ## Emit the explicit bounded host-exec bridge activation plan
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap bridge plan --bundle-dir /tmp/cynic-host-exec

.PHONY: host-bridge-install
host-bridge-install: ## Apply the bounded host-exec bridge install plan; requires root privileges
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap bridge install --bundle-dir /tmp/cynic-host-exec --apply || test "$$?" = "1"

.PHONY: host-bridge-rootless-smoke
host-bridge-rootless-smoke: ## Execute the bounded host-exec bridge install chain in /tmp without root
	@$(RUST_ENV) $(CARGO) build -q -p cynic-host --bin cynic-host-exec
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap bridge rootless-smoke --sandbox-root /tmp/cynic-host-exec-rootless --apply

.PHONY: host-bridge-status
host-bridge-status: ## Observe whether the bounded host-exec bridge is active
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap bridge status || test "$$?" = "1"

.PHONY: local-host-principal-plan
local-host-principal-plan: ## Emit explicit runtime principal preparation plan
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap host principal --node local-kernel --registry $(LOCAL_REGISTRY)

.PHONY: local-host-principal-apply
local-host-principal-apply: ## Apply explicit runtime principal preparation; requires root privileges
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap host principal --node local-kernel --registry $(LOCAL_REGISTRY) --apply || test "$$?" = "1"

.PHONY: local-host-apply-report
local-host-apply-report: ## Emit the local host dry-run apply report without mutating this machine
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap host apply-report --node local-kernel --registry $(LOCAL_REGISTRY)

.PHONY: local-host-e2e
local-host-e2e: ## Execute the non-mutating local host substrate chain and report honest blockers
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap host e2e --node local-kernel --registry $(LOCAL_REGISTRY) --runtime-profile single-node --dry-run || test "$$?" = "1"

.PHONY: local-network-proof
local-network-proof: ## Observe local network runtime and emit network_runtime_observed proof bundle
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- network proof --registry $(LOCAL_REGISTRY) || test "$$?" = "1"

.PHONY: local-proof-plan
local-proof-plan: ## Generate local proofs under /tmp and feed them into runtime plan
	@tmpdir="$$(mktemp -d)"
	@echo "==> writing local proof bundles to $$tmpdir" >&2
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap host proof --node local-kernel --registry $(LOCAL_REGISTRY) > "$$tmpdir/host-proof.json" || test "$$?" = "1"
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- network proof --registry $(LOCAL_REGISTRY) > "$$tmpdir/network-proof.json" || test "$$?" = "1"
	@jq -s '{schema:"cynic.runtime_proof_bundle.v1", status: (if all(.[]; .status == "satisfied") then "satisfied" else "blocked" end), proofs: (map(.proofs) | add)}' "$$tmpdir/host-proof.json" "$$tmpdir/network-proof.json" > "$$tmpdir/proofs.json"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime plan --json --registry $(LOCAL_REGISTRY) --profile single-node --proof "$$tmpdir/proofs.json"

.PHONY: e2e
e2e: ## Execute the local single-node E2E proof chain without pretending blocked runtime is healthy
	@$(MAKE) --no-print-directory local-check >&2
	@tmpdir="$$(mktemp -d)"
	@echo "==> local single-node e2e artifacts: $$tmpdir" >&2
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime plan --json --registry $(LOCAL_REGISTRY) --profile single-node > "$$tmpdir/plan.initial.json"
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- bootstrap host proof --node local-kernel --registry $(LOCAL_REGISTRY) > "$$tmpdir/host-proof.json" || test "$$?" = "1"
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) $(CARGO) run -q -p cynic-cli -- network proof --registry $(LOCAL_REGISTRY) > "$$tmpdir/network-proof.json" || test "$$?" = "1"
	@jq -s '{schema:"cynic.runtime_proof_bundle.v1", status: (if all(.[]; .status == "satisfied") then "satisfied" else "blocked" end), proofs: (map(.proofs) | add)}' "$$tmpdir/host-proof.json" "$$tmpdir/network-proof.json" > "$$tmpdir/proofs.json"
	@$(RUST_ENV) $(CARGO) run -q -p cynic-cli -- runtime plan --json --registry $(LOCAL_REGISTRY) --profile single-node --proof "$$tmpdir/proofs.json" > "$$tmpdir/plan.with-proofs.json"
	@jq -e '.schema == "cynic.runtime_execution_plan.v1" and .profile == "single-node"' "$$tmpdir/plan.initial.json" >/dev/null
	@jq -e '.schema == "cynic.runtime_proof_bundle.v1" and (.proofs | length >= 1) and any(.proofs[]; .gate == "host_identity_observed")' "$$tmpdir/host-proof.json" >/dev/null
	@jq -e '.schema == "cynic.runtime_proof_bundle.v1" and (.proofs | length >= 1) and any(.proofs[]; .gate == "network_runtime_observed")' "$$tmpdir/network-proof.json" >/dev/null
	@jq -e '.schema == "cynic.runtime_proof_bundle.v1" and (.proofs | length >= 2)' "$$tmpdir/proofs.json" >/dev/null
	@jq -e '.schema == "cynic.runtime_execution_plan.v1" and .profile == "single-node"' "$$tmpdir/plan.with-proofs.json" >/dev/null
	@jq -n \
		--arg artifacts "$$tmpdir" \
		--slurpfile host "$$tmpdir/host-proof.json" \
		--slurpfile network "$$tmpdir/network-proof.json" \
		--slurpfile plan "$$tmpdir/plan.with-proofs.json" \
		'{schema:"cynic.local_e2e.v1", status:"ok", artifacts:$$artifacts, host_proof_status:$$host[0].status, network_proof_status:$$network[0].status, plan_status:$$plan[0].status, observed_gates:$$plan[0].observed_gates, first_blocked_layer: ($$plan[0].layers | map(select(.status == "blocked")) | first // null)}'

.PHONY: build-admission-check-build
build-admission-check-build:
	@CYNIC_BUILD_ADMISSION_GRANT="$(CYNIC_BUILD_ADMISSION_GRANT)" \
		CYNIC_BUILD_WORKLOAD_RECORD="$(CYNIC_BUILD_WORKLOAD_RECORD)" \
		CYNIC_BUILD_AUTHORITY_PUBLIC_KEY="$(CYNIC_BUILD_AUTHORITY_PUBLIC_KEY)" \
		CYNIC_BUILD_ADMISSION_VERIFIER="$(CYNIC_BUILD_ADMISSION_VERIFIER)" \
		CYNIC_BUILD_BOOTSTRAP_RECEIPT="$(CYNIC_BUILD_BOOTSTRAP_RECEIPT)" \
		./scripts/build-admission-check make:build

.PHONY: build-admission-check-release
build-admission-check-release:
	@CYNIC_BUILD_ADMISSION_GRANT="$(CYNIC_BUILD_ADMISSION_GRANT)" \
		CYNIC_BUILD_WORKLOAD_RECORD="$(CYNIC_BUILD_WORKLOAD_RECORD)" \
		CYNIC_BUILD_AUTHORITY_PUBLIC_KEY="$(CYNIC_BUILD_AUTHORITY_PUBLIC_KEY)" \
		CYNIC_BUILD_ADMISSION_VERIFIER="$(CYNIC_BUILD_ADMISSION_VERIFIER)" \
		CYNIC_BUILD_BOOTSTRAP_RECEIPT="$(CYNIC_BUILD_BOOTSTRAP_RECEIPT)" \
		./scripts/build-admission-check make:build-release

.PHONY: build
build: build-admission-check-build ## Experimental binary build; on 2026-07-16 rustc 1.96.0 still shows workspace instability here
	@$(BUILD_RUST_ENV) $(CARGO) build -p cynic-kernel -p cynic-node -p cynic-mcp -p cynic-cli

.PHONY: build-release
build-release: build-admission-check-release ## Admitted release build on the registry-selected build runner
	@$(RELEASE_RUST_ENV) $(CARGO) build -p cynic-host -p cynic-kernel -p cynic-node -p cynic-mcp -p cynic-cli --release

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

.PHONY: run-mcp
run-mcp: ## Run the MCP proxy locally over stdio
	@$(RUST_ENV) $(CARGO) run -p cynic-mcp

.PHONY: install-user-bins
install-user-bins: build ## Install release binaries into ~/bin
	@mkdir -p "$$HOME/bin"
	@install -m 0755 "$(CARGO_TARGET_DIR)/debug/cynic-kernel" "$$HOME/bin/cynic-kernel"
	@install -m 0755 "$(CARGO_TARGET_DIR)/debug/cynic-node" "$$HOME/bin/cynic-node"
	@install -m 0755 "$(CARGO_TARGET_DIR)/debug/cynic-mcp" "$$HOME/bin/cynic-mcp"
	@install -m 0755 "$(CARGO_TARGET_DIR)/debug/cynic" "$$HOME/bin/cynic"

.PHONY: systemd-list
systemd-list: ## List remaining runtime unit templates
	@find infra/systemd -maxdepth 1 -type f | sort
