SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.NOTPARALLEL:

PROJECT_DIR := $(shell git rev-parse --show-toplevel 2>/dev/null || pwd)
CORE_CRATES := cynic-protocol cynic-host cynic-node cynic-kernel cynic-mcp cynic-cli cynic-askesis
LAB_REGISTRY := infra/registries/lab.json
LOCAL_REGISTRY := infra/registries/single-node.json
RUST_ENV := RUST_MIN_STACK=268435456 CARGO_INCREMENTAL=0
BUILD_RUST_ENV := RUST_MIN_STACK=536870912 CARGO_INCREMENTAL=0
RELEASE_RUST_ENV := RUST_MIN_STACK=1073741824 CARGO_INCREMENTAL=0
TEST_RUST_ENV := RUST_MIN_STACK=268435456 CARGO_INCREMENTAL=0

.PHONY: check
check: ## Validate the living low-layer Rust runtime baseline
	@echo "==> checking foundation, runtime authority, and registry"
	@jq empty contracts/foundation.json
	@jq empty infra/runtime-authority.json
	@jq empty infra/registry.json
	@jq empty infra/agent-exposure.json
	@echo "==> cargo check core crates"
	@$(RUST_ENV) cargo check $(foreach crate,$(CORE_CRATES),-p $(crate))
	@$(MAKE) --no-print-directory surface-audit

.PHONY: surface-audit
surface-audit: ## Verify canonical CLI surfaces are backed by real modules and mutation policies
	@echo "==> auditing canonical CLI surfaces"
	@$(RUST_ENV) cargo run -q -p cynic-cli -- dev surface-audit --json | jq -e '.schema == "cynic.cli.surface_audit.v1" and .status == "ok" and .blocked_count == 0' >/dev/null

.PHONY: lab-check
lab-check: ## Validate portable lab registry and low-layer crates used by lab smoke tests
	@echo "==> checking portable lab registry"
	@jq empty $(LAB_REGISTRY)
	@echo "==> cargo check lab-facing crates"
	@$(RUST_ENV) cargo check -p cynic-host -p cynic-node -p cynic-cli

.PHONY: local-check
local-check: ## Validate single-node registry and local low-layer crates
	@echo "==> checking single-node registry"
	@jq empty $(LOCAL_REGISTRY)
	@echo "==> cargo check local-facing crates"
	@$(RUST_ENV) cargo check -p cynic-host -p cynic-node -p cynic-cli

.PHONY: lab-smoke
lab-smoke: lab-check ## Prove lab registry can drive N-case DNS and bootstrap projections without real VMs
	@tmpdir="$$(mktemp -d)"
	@echo "==> generating closed lab VM plan from $(LAB_REGISTRY)"
	@$(RUST_ENV) cargo run -q -p cynic-cli -- lab plan --json --registry $(LAB_REGISTRY) > "$$tmpdir/lab-plan.json"
	@jq -e '.schema == "cynic.lab.plan.v1" and .mutates == false and .can_apply == false and .vm_count == 3 and .isolation.host_routing_mutation == false' "$$tmpdir/lab-plan.json" >/dev/null
	@echo "==> generating closed lab VM bundle from $(LAB_REGISTRY)"
	@$(RUST_ENV) cargo run -q -p cynic-cli -- lab bundle --json --registry $(LAB_REGISTRY) --output-dir "$$tmpdir/lab-bundle" > "$$tmpdir/lab-bundle.json"
	@jq -e '.schema == "cynic.lab.bundle.v1" and .mutates == false and .vm_count == 3' "$$tmpdir/lab-bundle.json" >/dev/null
	@test -f "$$tmpdir/lab-bundle/manifest.json"
	@test -f "$$tmpdir/lab-bundle/network.xml"
	@echo "==> generating DNS bundle from $(LAB_REGISTRY)"
	@$(RUST_ENV) cargo run -q -p cynic-cli -- network dns bundle --registry $(LAB_REGISTRY) --output-dir "$$tmpdir/dns" > "$$tmpdir/dns-report.json"
	@grep -R "labmesh.example" "$$tmpdir/dns" >/dev/null
	@if grep -R "synapse.cynic" "$$tmpdir/dns" >/dev/null; then echo "lab DNS bundle leaked default synapse.cynic"; exit 1; fi
	@echo "==> checking bootstrap projection from $(LAB_REGISTRY)"
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap constellation --json --registry $(LAB_REGISTRY) > "$$tmpdir/bootstrap.json" || test "$$?" = "1"
	@jq -e '.status == "bootstrap_required" and (.nodes | length == 3)' "$$tmpdir/bootstrap.json" >/dev/null
	@echo "lab smoke ok: $$tmpdir"

.PHONY: lab-plan
lab-plan: ## Emit the non-mutating closed VM lab plan
	@$(RUST_ENV) cargo run -q -p cynic-cli -- lab plan --json --registry $(LAB_REGISTRY)

.PHONY: lab-provision
lab-provision: ## Run closed VM lab provision preflight; refuses mutation until registry prerequisites are declared
	@$(RUST_ENV) cargo run -q -p cynic-cli -- lab provision --json --registry $(LAB_REGISTRY) --apply || test "$$?" = "1"

.PHONY: lab-bundle
lab-bundle: ## Generate non-mutating closed VM lab libvirt/cloud-init bundle under /tmp
	@tmpdir="$$(mktemp -d)"
	@$(RUST_ENV) cargo run -q -p cynic-cli -- lab bundle --json --registry $(LAB_REGISTRY) --output-dir "$$tmpdir"

.PHONY: framework-bootstrap-bundle
framework-bootstrap-bundle: ## Generate framework bootstrap kit for a fresh Linux node
	@tmpdir="$$(mktemp -d)"
	@$(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap framework bundle --json --output-dir "$$tmpdir"

.PHONY: lab-guest-bootstrap-bundle
lab-guest-bootstrap-bundle: framework-bootstrap-bundle ## Compatibility alias for the closed lab bootstrap usecase

.PHONY: lab-image-plan
lab-image-plan: ## Emit base image download/checksum preflight without downloading
	@$(RUST_ENV) cargo run -q -p cynic-cli -- lab image plan --json --registry $(LAB_REGISTRY)

.PHONY: lab-image-resolve
lab-image-resolve: ## Resolve expected base image sha256 from upstream SHA256SUMS without editing registry
	@$(RUST_ENV) cargo run -q -p cynic-cli -- lab image resolve --json --registry $(LAB_REGISTRY)

.PHONY: lab-image-fetch
lab-image-fetch: ## Refuse image fetch until registry has a pinned sha256
	@$(RUST_ENV) cargo run -q -p cynic-cli -- lab image fetch --json --registry $(LAB_REGISTRY) --apply || test "$$?" = "1"

.PHONY: runtime-validate
runtime-validate: ## Execute read-only CLI runtime validation against fleet + portable lab registries
	@$(MAKE) --no-print-directory lab-check >&2
	@echo "==> executing runtime validation" >&2
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- runtime validate --json --registry infra/registry.json --lab-registry $(LAB_REGISTRY)

.PHONY: runtime-suite
runtime-suite: ## Emit the stable IaC execution suite for the portable lab registry
	@$(RUST_ENV) cargo run -q -p cynic-cli -- runtime suite --json --registry $(LAB_REGISTRY) --profile lab

.PHONY: runtime-plan
runtime-plan: ## Emit the current non-mutating execution plan for the portable lab registry
	@$(RUST_ENV) cargo run -q -p cynic-cli -- runtime plan --json --registry $(LAB_REGISTRY) --profile lab

.PHONY: runtime-harness
runtime-harness: ## Emit the canonical runtime harness progression and execution membranes
	@$(RUST_ENV) cargo run -q -p cynic-cli -- runtime harness --json

.PHONY: runtime-conformance
runtime-conformance: ## Observe host conformance for this node without mutating host/network state
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- runtime conformance --json --registry $(LOCAL_REGISTRY) || test "$$?" = "1"

.PHONY: runtime-metrics
runtime-metrics: ## Emit percent-based runtime metrics; unknown metrics remain null
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- runtime metrics --json --registry $(LOCAL_REGISTRY)

.PHONY: runtime-metrics-architecture
runtime-metrics-architecture: ## Run architecture boundary tests and consume their proof in runtime metrics
	@tmpdir="$$(mktemp -d)"
	@$(TEST_RUST_ENV) cargo test -q -p cynic-cli --test architecture_boundary >&2
	@jq -n '{schema:"cynic.runtime_proof_bundle.v1",status:"satisfied",proofs:[{schema:"cynic.runtime_proof.v1",gate:"architecture_boundary_verified",status:"satisfied",producer:"cargo-test",subject:"cynic-cli/tests/architecture_boundary.rs",observed_at_unix_ms:1,evidence:[{id:"architecture_boundary",status:"active",detail:"cargo test -p cynic-cli --test architecture_boundary passed"}]}]}' > "$$tmpdir/architecture-proof.json"
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- runtime metrics --json --registry $(LOCAL_REGISTRY) --proof "$$tmpdir/architecture-proof.json"

.PHONY: local-plan
local-plan: ## Emit the current non-mutating execution plan for this single machine
	@$(RUST_ENV) cargo run -q -p cynic-cli -- runtime plan --json --registry $(LOCAL_REGISTRY) --profile single-node

.PHONY: local-host-proof
local-host-proof: ## Observe this host and emit host_identity_observed proof bundle
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap host proof --node local-kernel --registry $(LOCAL_REGISTRY) || test "$$?" = "1"

.PHONY: local-host-plan
local-host-plan: ## Emit the local host bootstrap plan without mutating this machine
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap host plan --node local-kernel --registry $(LOCAL_REGISTRY)

.PHONY: host-bridge-bundle
host-bridge-bundle: ## Build the bounded host-exec bridge bundle under /tmp
	@$(RUST_ENV) cargo build -q -p cynic-host --bin cynic-host-exec
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap bridge bundle --output-dir /tmp/cynic-host-exec

.PHONY: host-bridge-plan
host-bridge-plan: ## Emit the explicit bounded host-exec bridge activation plan
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap bridge plan --bundle-dir /tmp/cynic-host-exec

.PHONY: host-bridge-install
host-bridge-install: ## Apply the bounded host-exec bridge install plan; requires root privileges
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap bridge install --bundle-dir /tmp/cynic-host-exec --apply || test "$$?" = "1"

.PHONY: host-bridge-rootless-smoke
host-bridge-rootless-smoke: ## Execute the bounded host-exec bridge install chain in /tmp without root
	@$(RUST_ENV) cargo build -q -p cynic-host --bin cynic-host-exec
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap bridge rootless-smoke --sandbox-root /tmp/cynic-host-exec-rootless --apply

.PHONY: host-bridge-status
host-bridge-status: ## Observe whether the bounded host-exec bridge is active
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap bridge status || test "$$?" = "1"

.PHONY: local-host-principal-plan
local-host-principal-plan: ## Emit explicit runtime principal preparation plan
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap host principal --node local-kernel --registry $(LOCAL_REGISTRY)

.PHONY: local-host-principal-apply
local-host-principal-apply: ## Apply explicit runtime principal preparation; requires root privileges
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap host principal --node local-kernel --registry $(LOCAL_REGISTRY) --apply || test "$$?" = "1"

.PHONY: local-host-apply-report
local-host-apply-report: ## Emit the local host dry-run apply report without mutating this machine
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap host apply-report --node local-kernel --registry $(LOCAL_REGISTRY)

.PHONY: local-host-e2e
local-host-e2e: ## Execute the non-mutating local host substrate chain and report honest blockers
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap host e2e --node local-kernel --registry $(LOCAL_REGISTRY) --runtime-profile single-node --dry-run || test "$$?" = "1"

.PHONY: local-network-proof
local-network-proof: ## Observe local network runtime and emit network_runtime_observed proof bundle
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- network proof --registry $(LOCAL_REGISTRY) || test "$$?" = "1"

.PHONY: local-proof-plan
local-proof-plan: ## Generate local proofs under /tmp and feed them into runtime plan
	@tmpdir="$$(mktemp -d)"
	@echo "==> writing local proof bundles to $$tmpdir" >&2
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap host proof --node local-kernel --registry $(LOCAL_REGISTRY) > "$$tmpdir/host-proof.json" || test "$$?" = "1"
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- network proof --registry $(LOCAL_REGISTRY) > "$$tmpdir/network-proof.json" || test "$$?" = "1"
	@jq -s '{schema:"cynic.runtime_proof_bundle.v1", status: (if all(.[]; .status == "satisfied") then "satisfied" else "blocked" end), proofs: (map(.proofs) | add)}' "$$tmpdir/host-proof.json" "$$tmpdir/network-proof.json" > "$$tmpdir/proofs.json"
	@$(RUST_ENV) cargo run -q -p cynic-cli -- runtime plan --json --registry $(LOCAL_REGISTRY) --profile single-node --proof "$$tmpdir/proofs.json"

.PHONY: e2e
e2e: ## Execute the local single-node E2E proof chain without pretending blocked runtime is healthy
	@$(MAKE) --no-print-directory local-check >&2
	@tmpdir="$$(mktemp -d)"
	@echo "==> local single-node e2e artifacts: $$tmpdir" >&2
	@$(RUST_ENV) cargo run -q -p cynic-cli -- runtime plan --json --registry $(LOCAL_REGISTRY) --profile single-node > "$$tmpdir/plan.initial.json"
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- bootstrap host proof --node local-kernel --registry $(LOCAL_REGISTRY) > "$$tmpdir/host-proof.json" || test "$$?" = "1"
	@CYNIC_HOST_PROBE_TIMEOUT_MS=200 $(RUST_ENV) cargo run -q -p cynic-cli -- network proof --registry $(LOCAL_REGISTRY) > "$$tmpdir/network-proof.json" || test "$$?" = "1"
	@jq -s '{schema:"cynic.runtime_proof_bundle.v1", status: (if all(.[]; .status == "satisfied") then "satisfied" else "blocked" end), proofs: (map(.proofs) | add)}' "$$tmpdir/host-proof.json" "$$tmpdir/network-proof.json" > "$$tmpdir/proofs.json"
	@$(RUST_ENV) cargo run -q -p cynic-cli -- runtime plan --json --registry $(LOCAL_REGISTRY) --profile single-node --proof "$$tmpdir/proofs.json" > "$$tmpdir/plan.with-proofs.json"
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

.PHONY: build
build: ## Experimental binary build; rustc 1.96.1 currently ICEs on cynic-kernel in this workspace
	@$(BUILD_RUST_ENV) cargo build -p cynic-kernel -p cynic-node -p cynic-mcp -p cynic-cli

.PHONY: build-release
build-release: ## Experimental release build; rustc 1.96.1 currently ICEs on this workspace
	@$(RELEASE_RUST_ENV) cargo build -p cynic-kernel -p cynic-node -p cynic-mcp -p cynic-cli --release

.PHONY: test
test: ## Run tests for low-layer core crates
	@echo "warning: rustc 1.96.1 currently ICEs while compiling cynic-kernel tests in this workspace"
	@$(TEST_RUST_ENV) cargo test $(foreach crate,$(CORE_CRATES),-p $(crate))

.PHONY: kani-node-mesh
kani-node-mesh: ## Run Kani proofs for low-layer mesh planning invariants (requires `cargo kani setup`)
	@$(RUST_ENV) cargo kani -p cynic-node

.PHONY: run-kernel
run-kernel: ## Run the kernel locally
	@$(RUST_ENV) cargo run -p cynic-kernel

.PHONY: run-node
run-node: ## Run the node locally
	@$(RUST_ENV) cargo run -p cynic-node

.PHONY: run-mcp
run-mcp: ## Run the MCP proxy locally over stdio
	@$(RUST_ENV) cargo run -p cynic-mcp

.PHONY: install-user-bins
install-user-bins: build ## Install release binaries into ~/bin
	@mkdir -p "$$HOME/bin"
	@install -m 0755 target/debug/cynic-kernel "$$HOME/bin/cynic-kernel"
	@install -m 0755 target/debug/cynic-node "$$HOME/bin/cynic-node"
	@install -m 0755 target/debug/cynic-mcp "$$HOME/bin/cynic-mcp"
	@install -m 0755 target/debug/cynic "$$HOME/bin/cynic"

.PHONY: systemd-list
systemd-list: ## List remaining runtime unit templates
	@find infra/systemd -maxdepth 1 -type f | sort
