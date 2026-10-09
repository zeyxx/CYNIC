# Build-runner Harness Matrix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `cynic-builder` executable and provable through one registry-backed harness matrix without accepting a developer checkout or a local Cargo invocation as build evidence.

**Architecture:** `cynic-protocol` owns scenario types, validation, transitions, and evidence-status reduction. `cynic-host` projects registry scenarios into infrastructure observations and bounded plans through injected ports; `cynic-node` executes scheduler-owned conformance scenarios. The existing CLI conformance surface renders the same matrix and `cynic_validate` remains the only production validation dispatcher.

**Tech Stack:** Rust, Serde/JSON, existing `ArgvRunner` port, Proxmox/LXC adapter, libvirt lab projection, canonical CYNIC build workload protocol.

---

## File structure

| File | Responsibility |
| --- | --- |
| `crates/cynic-protocol/src/harness.rs` | Typed scenario contract, allowed states/actions, pure validation and reconciliation. |
| `crates/cynic-protocol/src/lib.rs` | Public protocol exports only. |
| `infra/registry.json` | Declarative scenario catalog for the registry-selected builder; no host commands. |
| `crates/cynic-host/src/build_runner_harness/{mod,types,registry,observe,plan}.rs` | Read registry data, inject observations, generate non-mutating plans, and reduce scenario evidence. |
| `crates/cynic-host/src/adapters/proxmox_clone.rs` | Disposable LXC clone/teardown adapter behind typed host ports. |
| `crates/cynic-host/src/lab.rs` | Project the pre-existing VM lab through `closed_lab_vm`; retain current lab behavior. |
| `crates/cynic-node/src/usecases/build_scheduler/conformance.rs` | Schedule preflight, live workload proof, cancellation/recovery and replay checks. |
| `crates/cynic-node/src/usecases/build_scheduler/{mod,evidence}.rs` | Wire conformance reports to scheduler evidence without duplicating scheduler policy. |
| `crates/cynic-host/src/vertical_conformance.rs` | Merge harness cells into the existing vertical conformance report. |
| `crates/cynic-cli/src/runtime_surface/{harness_matrix,conformance}.rs` | Thin parse/delegate/render glue for `cynic runtime conformance --json`. |

## Task 1: Define the protocol-owned harness contract

**Files:**
- Create: `crates/cynic-protocol/src/harness.rs`
- Modify: `crates/cynic-protocol/src/lib.rs`
- Test: `crates/cynic-protocol/src/harness.rs`

- [ ] **Step 1: Write failing pure-contract tests**

Add tests for these observable contracts:

```rust
#[test]
fn canary_requires_completed_staging_evidence() {
    let scenario = HarnessScenario::builder_canary_apply();
    let report = reconcile_harness_scenario(&scenario, &HarnessObservation::default());
    assert_eq!(report.status, HarnessStatus::Blocked);
    assert!(report.blockers.contains(&"staging_proof_missing".to_string()));
}

#[test]
fn desired_state_cannot_promote_unknown_proof() {
    let scenario = HarnessScenario::closed_lab_vm();
    let report = reconcile_harness_scenario(&scenario, &HarnessObservation::default());
    assert_eq!(report.status, HarnessStatus::Unknown);
}

#[test]
fn terminal_failure_preserves_evidence_reference() {
    let state = reduce_harness_evidence(&[HarnessEvidence {
        kind: HarnessEvidenceKind::Execution,
        status: HarnessEvidenceStatus::Failed,
        reference: "completion:sha256:deadbeef".to_string(),
    }]);
    assert_eq!(state.status, HarnessStatus::Failed);
    assert_eq!(state.evidence_refs, ["completion:sha256:deadbeef"]);
}
```

- [ ] **Step 2: Run the protocol test request and confirm it is initially red**

Request a remote check through `cynic_validate` with `action:"check"`, the current immutable source commit, and `request_id:"harness-contract-red"`. Expected: compilation failure because `HarnessScenario`, `HarnessObservation`, and `reconcile_harness_scenario` do not exist.

- [ ] **Step 3: Implement the contract in one focused module**

Define and serialize these stable items:

```rust
pub const HARNESS_SCENARIO_SCHEMA: &str = "cynic.build.harness_scenario.v1";
pub const HARNESS_REPORT_SCHEMA: &str = "cynic.build.harness_report.v1";

pub enum HarnessScenarioId {
    ClosedLabVm,
    LxcCloneConformance,
    BuilderStagingObserve,
    BuilderCanaryApply,
    FailureRecovery,
    ProofReplay,
}

pub enum HarnessStatus {
    Planned, Blocked, Unknown, Admitted, Claimed, Running,
    Completed, Failed, Cancelled, Stale,
}

pub enum HarnessAction { Observe, Clone, Destroy, DispatchCheck, InjectFault, Replay }

pub struct HarnessScenario {
    pub schema: String,
    pub id: HarnessScenarioId,
    pub authority: HarnessAuthority,
    pub target_role: String,
    pub prerequisites: Vec<HarnessPrerequisite>,
    pub allowed_actions: Vec<HarnessAction>,
    pub rollback: HarnessRollback,
}
```

Make `validate_harness_scenario` reject an empty target role, an action not permitted by the scenario, a mutation under `BuilderStagingObserve` or `ProofReplay`, and `BuilderCanaryApply` without a completed staging proof. Make `reconcile_harness_scenario` pure: it returns desired/observed/action/evidence/blockers/unknowns and never executes an adapter.

- [ ] **Step 4: Export the narrow public protocol surface**

Add `pub mod harness;` and re-export only the scenario inputs, report outputs, and pure functions required by host/node crates. Do not export serialization helpers or test fixtures.

- [ ] **Step 5: Verify the contract remotely**

Send `cynic_validate` a `check` request for the source commit with evidence reference `plan:build-runner-harness-contract`; retain the workload ID and report its terminal proof or explicit blocked state. A local Cargo result is not accepted as this task’s build evidence.

## Task 2: Add the registry-backed scenario catalog

**Files:**
- Modify: `infra/registry.json`
- Test: `crates/cynic-protocol/src/harness.rs`

- [ ] **Step 1: Add failing catalog-validation tests**

Validate a parsed fixture with all six IDs, then assert that malformed catalog data fails:

```rust
assert!(parse_harness_scenarios(valid_catalog()).is_ok());
assert_eq!(parse_harness_scenarios(missing_canary_rollback()),
           Err("harness_scenario_rollback_missing".to_string()));
assert_eq!(parse_harness_scenarios(canary_with_auto_apply()),
           Err("harness_canary_requires_explicit_apply".to_string()));
```

- [ ] **Step 2: Extend `infra/registry.json` under the existing build-runner declaration**

Add a `build_runner.harness_scenarios` array, not a new top-level authority. Each entry names `id`, `target_role`, `authority`, `enabled`, `allowed_actions`, `prerequisites`, `rollback`, and `required_proofs`. Use exactly the approved IDs. `builder_staging_observe` and `proof_replay` allow only `observe` and `replay`; `builder_canary_apply` contains `dispatch_check` and `explicit_apply_required:true`; no entry contains argv, SSH host syntax, paths, or a provider command.

- [ ] **Step 3: Parse the catalog into `HarnessScenario` values**

Implement `parse_harness_scenarios(&serde_json::Value) -> Result<Vec<HarnessScenario>, String>` in the protocol module. Normalize duplicate IDs to `harness_scenario_id_duplicate`; require one scenario for each approved ID; require a non-empty rollback declaration for mutating scenarios.

- [ ] **Step 4: Verify schema and authority behavior**

Run the canonical registry validation request, then assert in protocol tests that registry-enabled data does not cause a scenario to become `Completed` without observed evidence.

## Task 3: Build the host-side non-mutating matrix projection

**Files:**
- Create: `crates/cynic-host/src/build_runner_harness/mod.rs`
- Create: `crates/cynic-host/src/build_runner_harness/types.rs`
- Create: `crates/cynic-host/src/build_runner_harness/registry.rs`
- Create: `crates/cynic-host/src/build_runner_harness/observe.rs`
- Create: `crates/cynic-host/src/build_runner_harness/plan.rs`
- Modify: `crates/cynic-host/src/lib.rs`
- Test: `crates/cynic-host/src/build_runner_harness/plan.rs`

- [ ] **Step 1: Write host projection tests with fake ports**

Use fake `NodeSubstrateObservationPort`, `ServiceSupervisorObservationPort`, and source-transport observer inputs. Assert:

```rust
assert_eq!(report.cells.len(), 6);
assert_eq!(cell(&report, HarnessScenarioId::BuilderStagingObserve).status,
           HarnessStatus::Unknown);
assert!(cell(&report, HarnessScenarioId::BuilderStagingObserve)
    .unknowns.contains(&"provider_observation_unavailable".to_string()));
assert!(!report.mutates);
assert!(!report.can_apply);
```

- [ ] **Step 2: Implement focused host modules**

`registry.rs` reads only `infra/registry.json` and calls the protocol parser. `observe.rs` maps injected node/service/source observations into `HarnessObservation`; unavailable ports become explicit unknowns. `plan.rs` calls `reconcile_harness_scenario` for each catalog cell and emits `BuildRunnerHarnessReport { schema, desired, cells, blockers, residual_unknowns, mutates:false, can_apply:false }`. The facade only composes modules.

- [ ] **Step 3: Integrate with the existing runtime harness without growing its ownership**

Expose `pub mod build_runner_harness;` from `cynic-host`. Keep `runtime_harness.rs` as the generic kind/context catalog; call the new report from its conformance consumer rather than adding scenario policy to that file.

- [ ] **Step 4: Verify host projection remotely**

Submit a canonical `check` workload referencing `plan:host-harness-projection`. Expected terminal evidence: the host crate compiles and its harness projection tests run in the registry-selected build runner, or the report remains explicitly blocked by absent runner admission.

## Task 4: Project existing VM lab and add disposable LXC-clone conformance

**Files:**
- Modify: `crates/cynic-host/src/lab.rs`
- Create: `crates/cynic-host/src/adapters/proxmox_clone.rs`
- Modify: `crates/cynic-host/src/adapters/mod.rs`
- Test: `crates/cynic-host/src/adapters/proxmox_clone.rs`

- [ ] **Step 1: Write adapter tests against the existing `ArgvRunner` fake**

Verify exact bounded command vectors and rollback semantics:

```rust
assert_eq!(runner.calls(), vec![
    vec!["ssh", "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=yes",
         "proxmox", "sudo", "-n", "/usr/bin/pct", "clone", "105", "900105",
         "--full", "0", "--target", "pve"],
]);
assert_eq!(adapter.destroy_clone(&clone).unwrap_err(), "clone_read_back_missing");
```

Use non-real IDs and host names in fixtures; the production implementation receives the provider target and resource IDs through typed input.

- [ ] **Step 2: Implement `ProxmoxSshCloneAdapter`**

Inject `ArgvRunner` and SSH target; validate numeric LXC source/clone IDs and reject argument-like inputs. Implement `plan_clone`, `apply_clone`, `observe_clone`, and `destroy_clone`. `apply_clone` returns a typed outcome only after clone identity and isolated-network observation are read back; `destroy_clone` requires a matching clone identity. No shell strings and no implicit cleanup.

- [ ] **Step 3: Map the current VM lab to `closed_lab_vm`**

Reuse `build_lab_plan` and `build_lab_provision_report` as the VM scenario’s adapter input. Preserve their non-mutating default and existing host-network rules. A missing libvirt image remains a `Blocked` cell, not a lab success.

- [ ] **Step 4: Emit LXC teardown evidence into the same matrix**

Map clone creation/read-back/teardown to the `lxc_clone_conformance` scenario. A provider unavailability, clone mismatch, or teardown failure must produce `Unknown`, `Failed`, or `Stale` with evidence refs; it must never fall through to `Completed`.

- [ ] **Step 5: Verify VM and clone contracts**

Request canonical check execution with evidence ref `plan:vm-lxc-harness`. Preserve the execution report and only schedule the real LXC conformance scenario after a reviewed explicit apply request.

## Task 5: Add scheduler-owned staging, canary, failure, and replay cells

**Files:**
- Create: `crates/cynic-node/src/usecases/build_scheduler/conformance.rs`
- Modify: `crates/cynic-node/src/usecases/build_scheduler/mod.rs`
- Modify: `crates/cynic-node/src/usecases/build_scheduler/evidence.rs`
- Test: `crates/cynic-node/src/usecases/build_scheduler/conformance.rs`

- [ ] **Step 1: Write transition tests with local fake stores and fake executor**

Cover: staging observes but never dispatches; canary requires the staging proof; a cancellation becomes durable; expired claim is `Stale`; replay detects a changed evidence digest.

```rust
assert_eq!(plan_builder_staging(&input).action, HarnessAction::Observe);
assert_eq!(plan_builder_canary(&without_staging).status, HarnessStatus::Blocked);
assert_eq!(reduce_completion(&cancelled_completion).status, HarnessStatus::Cancelled);
assert_eq!(replay_proof(&different_digest).status, HarnessStatus::Stale);
```

- [ ] **Step 2: Implement only composition around existing scheduler primitives**

`conformance.rs` uses `BuildSchedulerConfig`, durable workload/grant stores, cancellation token, workspace observer, and `TokioBuildExecution` through their existing ports. It must not create a second queue, signature policy, or process runner. The canary action composes exactly one `make:check` admitted workload; the executor report must retain workload ID, signed grant reference, exact argv, workspace commit, exit status, and completion reference.

- [ ] **Step 3: Persist scenario evidence through the existing observer**

Add an evidence helper that writes the scenario ID, terminal state, and content-addressed references through `agent_execution_runtime_observer::persist_capabilities`. If writing fails, the report retains `node_owned_agent_execution_proof_unavailable`; no status promotion occurs.

- [ ] **Step 4: Verify scheduler conformance through the canonical dispatcher**

Use `cynic_validate` with `action:"check"`, a content-addressed source snapshot when available, and evidence ref `plan:scheduler-harness-conformance`. Do not invoke Cargo locally. Expected outcome is a durable workload receipt, later terminal completion, or a specific admission/runtime blocker.

## Task 6: Render the matrix in vertical conformance and CLI

**Files:**
- Modify: `crates/cynic-host/src/vertical_conformance.rs`
- Create: `crates/cynic-cli/src/runtime_surface/harness_matrix.rs`
- Modify: `crates/cynic-cli/src/runtime_surface/conformance.rs`
- Modify: `crates/cynic-cli/src/runtime_surface.rs`
- Test: `crates/cynic-host/src/vertical_conformance.rs`

- [ ] **Step 1: Add a failing report-shape test**

Assert the JSON projection includes all cells and does not conflate a partial matrix with success:

```rust
assert_eq!(report.build_runner_harness.cells.len(), 6);
assert_eq!(report.status, "blocked");
assert!(report.residual_unknowns.contains(&
    "node_owned_agent_execution_observer_not_wired".to_string()));
```

- [ ] **Step 2: Extend `VerticalConformanceReport` with a typed harness field**

Consume the host report rather than recomputing decisions. Map overall status with the existing convergence semantics: `Completed` only when all required cells are complete; unresolved capability observations remain residual unknowns.

- [ ] **Step 3: Add thin CLI delegation**

Add `runtime_surface::harness_matrix::run` that resolves registry/runtime layout, invokes host conformance assembly, and serializes existing `cli_json` output. The `conformance` command delegates to it. The CLI must not read Proxmox, issue SSH, or encode scenario policy.

- [ ] **Step 4: Verify the actual JSON surface**

After the build runner returns a valid artifact, run:

```bash
cynic runtime conformance --json
```

Expected: a `cynic.vertical_conformance_report.v1` projection containing `build_runner_harness`, six scenario cells, action/evidence/blocker fields, and explicit unknowns. If the installed artifact is stale, record that exact mismatch instead of accepting source-only output.

## Task 7: Add end-to-end conformance proof and gate later recipe work

**Files:**
- Modify: `crates/cynic-host/src/vertical_conformance.rs`
- Modify: `crates/cynic-node/src/usecases/build_scheduler/conformance.rs`
- Test: existing unit modules plus an integration test in `crates/cynic-host/src/vertical_conformance.rs`

- [ ] **Step 1: Write a full fake-port integration test**

Construct a matrix where lab and clone are complete, builder observation is complete, canary has a matching live completion, fault recovery is terminal, and replay digest matches. Assert complete only then:

```rust
let report = assemble_vertical_conformance(fake_complete_inputs()).unwrap();
assert_eq!(report.build_runner_harness.status, HarnessStatus::Completed);
assert_eq!(report.status, "qualified");
```

Then alter one replay digest and assert `Stale` plus non-qualified overall status.

- [ ] **Step 2: Implement final status reduction without shortcuts**

Use the protocol reducer for every cell. Require evidence refs for all terminal mutating actions; require cancellation/expiry/source failure reports for `failure_recovery`; require exact digest equality for `proof_replay`. The incomplete real builder remains `Blocked` until its own staging and canary evidence exists.

- [ ] **Step 3: Run canonical verification and capture proof**

Dispatch a final `cynic_validate` `check` request for the immutable source commit with evidence refs `plan:build-runner-harness` and the registry digest. On terminal completion, run the installed `cynic runtime conformance --json`, retain both IDs/references, and compare desired registry digest with runtime registry digest. A mismatch leaves the result blocked.

- [ ] **Step 4: Unblock later recipe work only on proven runner capability**

The subsequent data-centric recipe phase may start only if the harness reports a real `builder_canary_apply` completion with matching signed grant, argv, workspace commit, and durable completion evidence. Otherwise retain the recipe phase’s current explicit blocked status.

## Plan review

- **Spec coverage:** Tasks 1–2 provide typed policy and registry validation; 3–4 cover VM/LXC host conformance; 5 covers staging/canary/failure/replay scheduler behavior; 6–7 render and prove the vertical surface.
- **No hidden authority:** all execution stays behind existing ports and `cynic_validate`; the catalog has no command strings and the CLI owns no decision tables.
- **Validation constraint:** the currently connected MCP artifact and `/etc/cynic/registry.json` are source-incompatible, so every plan step treats source compilation as unproven until a registry-selected build runner returns durable terminal evidence.
