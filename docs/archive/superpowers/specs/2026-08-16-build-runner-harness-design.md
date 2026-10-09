# Build-runner harness matrix

> Planning record only. Runtime authority remains `contracts/foundation.json`, `infra/runtime-authority.json`, `infra/registry.json`, and the canonical Rust crates.

## Goal

Make the canonical build runner executable and provable without treating a local agent checkout as build evidence. One data-driven harness matrix must reproduce isolated VM, disposable LXC, and real `cynic-builder` scenarios while preserving their distinct authorities.

## Desired state

`infra/registry.json` is the sole declarative source for target role, provider placement, source transport, resource limits, scenario enablement, bounded actions, and rollback policy. A scenario references registry roles and capability IDs; it does not embed machine-specific shell decisions.

`cynic-protocol` owns typed `HarnessScenario` input, pure validation/reconciliation, state transitions, and proof schemas. `cynic-host` owns infrastructure plans and observations through injected adapters. `cynic-node` owns scheduler claim, isolated workspace, signed grant, argv execution, and completion evidence. `cynic-kernel` owns admission, delivery, and durable read-back. CLI and MCP only project these reports.

## Scenario matrix

| ID | Authority | Bounded action | Required proof |
|---|---|---|---|
| `closed_lab_vm` | conformance gate | create/destroy isolated QEMU/libvirt guests | target identity, enrollment, scheduler observation, teardown read-back |
| `lxc_clone_conformance` | conformance gate | create/destroy isolated LXC clone through provider adapter | clone identity, isolated network, scheduler observation, rollback read-back |
| `builder_staging_observe` | staging observation | none | `cynic-builder` identity, enrollment, source transport, service state |
| `builder_canary_apply` | explicit canary only | one admitted `make:check` workload in an isolated workspace | live signed grant, exact argv, workspace commit, exit report, durable completion |
| `failure_recovery` | conformance gate | fault injection only in VM/clone scenarios | durable cancellation, expiry/reclaim, timeout, source-transport failure states |
| `proof_replay` | read-only projection | none | retained proof re-evaluation and explicit drift result |

A real builder always has two transitions: non-mutating `plan_observe`, then separately admitted `canary_apply`. No scenario enables automatic production mutation.

## Invariants

1. Every scenario follows `desired → observe → reconcile → bounded action → read-back proof`.
2. Each matrix cell is exactly one of `planned`, `blocked`, `unknown`, `admitted`, `claimed`, `running`, `completed`, `failed`, `cancelled`, or `stale`.
3. Desired state cannot promote a cell to success. Missing or stale proof remains data.
4. VM/clone proof establishes portability; it never promotes the real builder. The real builder requires its own staging/canary read-back.
5. All action parameters derive from an admitted typed scenario and registry data. Shell profiles and Makefile targets are compatibility projections, not policy owners.
6. Fault scenarios are first-class contracts. Every injected failure produces a durable terminal state and content-addressed evidence.

## Delivery order

1. Introduce typed scenario policy and registry validation with pure unit tests.
2. Project the existing VM lab as `closed_lab_vm`; add durable scheduler proof observation.
3. Add the LXC clone adapter behind the same port and prove cleanup/rollback.
4. Add real-builder observation only; retain all blockers as explicit data.
5. Permit one explicit, admitted `builder_canary_apply` after staging proof exists.
6. Add fault/replay matrix cases and require their reports in vertical conformance.
7. Resume the blocked data-centric build recipe phase only after an admitted runner can execute its test contract.

## Validation

- Unit tests cover scenario validation, transition preconditions, and proof-status reduction.
- A VM lab smoke test proves scheduler execution and cleanup without real-node authority.
- LXC clone conformance proves provider adapter behavior and teardown.
- Real-builder staging and canary use the canonical `cynic_validate` path; no local Cargo fallback is accepted as proof.
- `cynic runtime conformance --json` and convergence output show each scenario’s desired state, observed state, action, evidence references, blockers, and residual unknowns.
