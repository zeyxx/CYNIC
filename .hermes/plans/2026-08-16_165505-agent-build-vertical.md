# Agent Build Vertical Completion Plan

> **For Hermes:** execute bottom-up, one bounded slice at a time; do not use Cortex as a build fallback.

**Goal:** Close the admitted agent-session → delegated workload → fenced build-runner → durable evidence/PoH → kernel read-back → promotion vertical.

**Architecture:** `cynic-protocol` owns admissible states and receipt contracts. `cynic-node` owns builder-side claim, isolated source/build execution, evidence persistence, and durable workload revisions. `cynic-kernel` owns authoritative PoH append/read-back. CLI/MCP only project these results.

**Current facts (observed 2026-08-16):**
- Worktree-local Cargo caches are isolated but non-authoritative.
- Workload revisions are append-only and hash-linked locally.
- `degraded` and `recovered_unattributed` retain explicit unknowns/evidence and are non-promotable.
- Build provenance evidence now contains ownership, fence, repository, worktree/commit, argv, output digests, and prior workload-record digest.
- The scheduler is not deployed on the selected `build_runner`.
- `./scripts/cargo test ...` is blocked by missing `CYNIC_BUILD_ADMISSION_GRANT`; protocol exports reference missing modules (`freshness`, `agentic_tool_call`, `solana_ops`; Intel also references `solana_safety_dog`).

---

### Task 1: Freeze and validate the local degradation contract

**Objective:** Make the new terminal states impossible to promote and prove their validation rules.

**Files:**
- Modify only if tests reveal a gap: `crates/cynic-protocol/src/agent_recursion.rs`
- Modify only if tests reveal a gap: `crates/cynic-protocol/src/agent_workload/{types,lifecycle,durability}.rs`
- Test: `crates/cynic-protocol/src/agent_recursion/tests.rs`
- Test: `crates/cynic-protocol/src/agent_workload/tests.rs`

**Steps:**
1. Run formatting and diff integrity:
   `RUSTUP_TOOLCHAIN=1.96.0 rustfmt --edition 2024 --check <modified protocol files> && git diff --check`
2. Once local validation is admitted, run:
   `CYNIC_LOCAL_VALIDATION=1 CYNIC_RUST_MIN_STACK=268435456 CARGO_BUILD_JOBS=1 make check`
3. Add/keep tests proving:
   - degraded/recovered without unknowns are rejected;
   - degraded/recovered without content-addressed evidence are rejected;
   - neither status maps to `Completed` or a release-eligible state;
   - durable record state and completion status must agree.

**Acceptance:** All state transitions are machine-validated; no rendering or CLI path can treat degraded/recovered as success.

### Task 2: Repair the missing-module boundary without importing horizontal debt

**Objective:** Restore compilation only for protocol/Intel modules whose exports are currently dangling, after proving each public API is needed by an in-scope crate.

**Files to audit:**
- `crates/cynic-protocol/src/lib.rs`
- Consumers of `freshness`, `agentic_tool_call`, and `solana_ops`
- `crates/cynic-intel/src/lib.rs`
- Consumers of `solana_safety_dog`

**Steps:**
1. Classify each dangling export as one of: low-layer required protocol primitive, optional product-domain feature, or stale export.
2. For stale/optional exports, remove or feature-gate the export and its consumers instead of copying untracked laptop code.
3. For a proven required primitive, recreate the smallest typed contract with unit tests at its owning crate.
4. Run a targeted compile after each isolated repair using the approved local-validation path.

**Acceptance:** `cynic-protocol` and `cynic-node` compile without importing Solana/agentic policy modules merely to satisfy unrelated product surfaces.

### Task 3: Make build provenance a first-class typed receipt

**Objective:** Replace the current JSON evidence payload boundary with a typed protocol receipt shared by scheduler and kernel PoH.

**Files:**
- Extend: `crates/cynic-protocol/src/agent_execution_provenance.rs`
- Extend tests in the same module
- Adapt: `crates/cynic-node/src/usecases/build_scheduler/evidence.rs`
- Adapt: `crates/cynic-node/src/usecases/build_scheduler/mod.rs`

**Steps:**
1. Add typed build ownership/read-back metadata: session lineage when available, delegation/workload, node, lease/fence, repository/base commit, workspace/branch policy, argv/environment digests, output/artifact evidence, previous receipt/durable-record digest.
2. Add a receipt authority/disposition field with exactly: `attested`, `degraded_local_observation`, `recovered_unattributed`.
3. Validate that `attested` requires session lineage and kernel PoH read-back; degraded/recovered require explicit unknowns and are not promotion eligible.
4. Serialize that typed receipt as content-addressed scheduler evidence; do not create a parallel agent log.

**Acceptance:** The scheduler emits one schema-validated receipt whose required fields distinguish a local observation from a promotable execution.

### Task 4: Add atomic kernel PoH append and read-back

**Objective:** Turn local revision history into an authoritative kernel checkpoint without multi-writer forks.

**Files to inspect/modify:**
- `crates/cynic-kernel/src/storage/surreal/proof_of_history.rs`
- Kernel proof port/use-case owning append and read-back
- Corresponding kernel storage tests

**Steps:**
1. Define a PoH checkpoint input containing the typed build receipt digest and the previous authoritative checkpoint digest.
2. Implement a single-writer/CAS or transactional append that rejects a stale predecessor.
3. Read the newly appended checkpoint back by id/digest and verify its predecessor, receipt digest, writer identity, and timestamp.
4. Return explicit `degraded_local_observation` if kernel storage is unavailable or the CAS is rejected; retain local evidence and unknowns.
5. Prove concurrent/stale predecessor attempts cannot create two accepted successors.

**Acceptance:** Kernel PoH is append-only, fork-resistant at the persistence boundary, and produces a read-back receipt. A local chain alone never claims kernel authority.

### Task 5: Stitch session identity into workloads

**Objective:** Make the authenticated MCP/kernel session mandatory for an attested workload, without making MCP transport stateful.

**Files:**
- `crates/cynic-mcp/src/{kernel_session.rs,session_identity.rs}`
- Agent workload/delegation protocol owner under `crates/cynic-protocol/src/`
- Scheduler admission path under `crates/cynic-node/src/usecases/build_scheduler/`

**Steps:**
1. Propagate internal `session_id`, generation/fence, and agent identity from the kernel session to delegation/workload admission.
2. Reject an `attested` workload receipt if session, delegation, workload, lease, or fence linkage is missing/mismatched.
3. Permit local historical recovery only through `recovered_unattributed`, with no promotion path.
4. Test session revoke/fence increment invalidates a stale workload claim.

**Acceptance:** MCP remains stateless at transport, while every promotable build traces internally to one read-back session identity and fence.

### Task 6: Bootstrap and prove the registry-selected build runner

**Objective:** Activate the scheduler only on the configured `build_runner`, with a bounded grant and post-activation read-back.

**Files:**
- `infra/registry.json` and selected node registry entry (read-only authority inputs)
- `crates/cynic-host/src/framework_bootstrap/bundle/node_control.rs`
- `crates/cynic-node/src/{config,cli.rs,node_control_runtime.rs}`
- `infra/agent-exposure.json` if activation changes an exposed surface

**Steps:**
1. Generate the node-control bundle for the registry-selected builder; verify its role is `build_runner` and `build_scheduler` is enabled.
2. Apply only through the existing explicit bootstrap/apply surface with its required consent and target scope; do not hand-edit `/etc/cynic`.
3. Run node-control check/read-back on the builder and retain config, principal, service, scheduler capability, and registry-selection evidence.
4. Submit a minimal admitted canary workload with unique worktree/cache and a short bounded argv.
5. Verify claim, lease/fence, source read-back, build receipt, local durable revision chain, kernel PoH checkpoint/read-back, then terminal state.
6. Verify a release/promotion command rejects a degraded or recovered canary.

**Acceptance:** No Cortex process executes the workload; the builder is the only runner and all layers return machine-readable proofs.

### Task 7: Gate promotion on attested PoH and publish operational status

**Objective:** Ensure surfaces show live state but never collapse unknown/degraded into success.

**Files:**
- Release/promotion policy owner in `crates/cynic-protocol/src/runtime_release.rs`
- Relevant node/kernel report renderers
- `infra/agent-exposure.json` only if a tool/report is added or renamed

**Steps:**
1. Require `attested` receipt disposition plus matching kernel PoH read-back for release eligibility.
2. Render status, scope, owner, evidence refs, residual unknowns, and recovery procedure in JSON.
3. Test each outcome: completed-attested, degraded-local, blocked, failed, recovered-unattributed.

**Acceptance:** Operators and agents can observe ongoing work during degradation, but promotion is mechanically impossible without the required proof chain.

## Final validation gate

1. `CYNIC_LOCAL_VALIDATION=1 CYNIC_RUST_MIN_STACK=268435456 CARGO_BUILD_JOBS=1 make check`
2. Targeted tests for protocol, node scheduler, kernel PoH, and MCP session fencing.
3. One admitted build-runner canary with receipts and kernel PoH read-back.
4. One forced storage/publication failure proving `degraded` is retained and promotion is rejected.
5. `git diff --check` and focused `rustfmt --check`.

## Non-goals

- Do not deploy/build on Cortex as a scheduler fallback.
- Do not fake `CYNIC_BUILD_ADMISSION_GRANT` or kernel identity.
- Do not import untracked Solana/agentic modules solely to make the workspace compile.
- Do not commit/push until the user explicitly asks.
