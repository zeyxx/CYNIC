# CYNIC Correction Plan — From Theory to Industrial Baseline

## Purpose

This document maps every gap between the normative constitution (`agent-constitution.md`), the meta-harness target (`harness-conformance.md`), and the current Rust runtime. It is organized for execution in waves — each wave is independently valuable and produces read-back proof.

This is not a backlog. It is an **audit and unification program**. Each item names what to audit, what to unify, and what proof must be produced.

---

## Phase 0 — Audit Inventory (no code yet, read-only)

Before any unification, each subsystem needs a baseline audit that documents current behavior with citations to the current Rust source. These audits are read-only and produce JSON reports.

### Audit A — Coordination & Claims

**Scope :** How claims are created, held, expired, and released.

Files to audit :
- `crates/cynic-protocol/src/coordination_lease.rs`
- `crates/cynic-protocol/src/coordination_runtime.rs`
- `crates/cynic-protocol/src/coord.rs`
- `crates/cynic-host/src/local_autonomy_runtime.rs`
- `crates/cynic-node/src/usecases/build_scheduler/lifecycle.rs`

Questions to answer with file:line citations :
1. Is there any durable claim store, or are claims in-memory only?
2. How is TTL enforced — heartbeat, wall-clock check, or lease fencing token?
3. What happens when the store is unreachable — fail-closed or fail-open?
4. Does Cynic-preempts-Baseline work at runtime, or only in tests?
5. Can a claim be amplified without parent delegation?

**Deliverable :** `reports/audit-coordination-claims.md` — table of every claim lifecycle point with current behavior, gap, and constitution reference.

### Audit B — Authority & Harness

**Scope :** How harness kind/authority is declared, validated, and promoted.

Files to audit :
- `crates/cynic-host/src/runtime_harness.rs`
- `crates/cynic-protocol/src/harness.rs`
- `crates/cynic-cli/src/runtime_surface/host_checks.rs`
- `crates/cynic-cli/src/mask_surface.rs`

Questions :
1. Is `HarnessAuthority` used consistently across `harness.rs` (build) and `runtime_harness.rs` (runtime), or are they two separate enums?
2. Does the CLI ever surface harness kind to the operator, or is it internal only?
3. Is the promotion flow (lab → production) implemented anywhere, or only documented?
4. Can a citizen operate without declaring a harness?
5. What surfaces mutate real state without `explicit_apply_required = true`?

**Deliverable :** `reports/audit-authority-harness.md` — matrix of (surface × authority × harness × apply-gate) with gaps.

### Audit C — Proof Inventory

**Scope :** Every proof type, its schema, its producer, and its consumer.

Files to audit :
- `crates/cynic-protocol/src/harness.rs` (HarnessEvidence, HarnessCellReport, HarnessMatrixReport)
- `crates/cynic-host/src/runtime_harness.rs` (RuntimeHarnessTargetAttestation)
- `crates/cynic-protocol/src/build_proof.rs`
- `crates/cynic-protocol/src/evidence.rs`
- `crates/cynic-node/src/usecases/build_scheduler/release_publication.rs`
- `crates/cynic-host/src/kernel_tunnel.rs` (TunnelProof)
- `crates/cynic-askesis/src/agent_loop.rs` (AgentLoopAuditReport)

Questions :
1. How many distinct proof types exist? (target: enumerate)
2. Which proofs have a schema string and which don't?
3. Which proofs carry a time window and which are timeless?
4. Is there any proof that is consumed by a higher layer but produced by the same layer (self-attestation)?
5. Can any proof type be extended without breaking existing consumers (forward-compat)?

**Deliverable :** `reports/audit-proof-inventory.md` — table of every proof with schema, producer, consumer, forward-compat status.

### Audit D — Scope & Admission Gate

**Scope :** How `admit_mutation` is (or isn't) used across mutation surfaces.

Files to audit :
- `crates/cynic-protocol/src/agent_citizenship.rs` (admit_mutation)
- `crates/cynic-node/src/usecases/builder_http.rs`
- `crates/cynic-node/src/usecases/build_workload_submission_uc.rs`
- `crates/cynic-node/src/usecases/build_scheduler/mod.rs`
- `crates/cynic-node/src/usecases/release_webhook.rs`
- `crates/cynic-host/src/update_lifecycle/`

Questions :
1. How many mutation surfaces exist?
2. Which ones call `admit_mutation` (or equivalent) and which bypass it?
3. Is scope containment (Crate contains File) checked at every mutation surface, or only in tests?
4. Is budget monotonicity enforced, or only documented?
5. Which surfaces mutate without an explicit `--apply` or equivalent?

**Deliverable :** `reports/audit-scope-admission.md` — matrix of (mutation surface × gate × scope check × budget check × apply gate).

### Audit E — Budget & Time

**Scope :** How budgets are granted, consumed, and how time windows are validated.

Files to audit :
- `crates/cynic-protocol/src/agent_citizenship.rs` (AgentBudgets, budgets_within)
- `crates/cynic-protocol/src/coordination_lease.rs` (TTL, heartbeat)
- `crates/cynic-host/src/runtime_harness.rs` (validate at now_unix_ms)
- `crates/cynic-node/src/usecases/build_scheduler/lifecycle.rs`

Questions :
1. Is there any code path where a budget can be amplified without parent approval?
2. How is wall-clock injected — `chrono::Utc::now()` directly, or injectable?
3. What happens when a lease expires mid-execution — hard stop or graceful completion?
4. Are budget consumptions atomic with the mutation, or can a mutation succeed without decrementing?

**Deliverable :** `reports/audit-budget-time.md` — every (budget, TTL, time-window) validation point with current behavior.

---

## Phase 1 — Unification Targets

After audits, these are the unification targets that close the gaps. Each is independently shippable.

### U1 — Canonical Coordination Store

**Constitution ref :** §5 Admission Gate, §11 Non-Amplification
**Depends on :** Audit A

Target : A single durable claim store that every mutation surface consults before mutating.

Current state : coordination_lease.rs has the types, but no durable store. Claims exist in tests and in-memory runtime structures only.

Acceptance criteria :
- A mutation surface that cannot reach the store refuses with `AgentCoordinationStoreUnavailable`
- No claim can be held without a TTL and a heartbeat renewal path
- Two overlapping claims in the same mode resolve first-wins; Cynic preempts baseline
- Every claim record carries: holder, scope, mode, budget, issued_at, expires_at, parent_claim_id

### U2 — Authority Enum Unification

**Constitution ref :** §5 Admission Gate, §7 Harness Binding
**Depends on :** Audit B

Target : A single `HarnessAuthority` enum shared by build harness and runtime harness. Currently `harness.rs` defines its own `HarnessAuthority` and `runtime_harness.rs` defines a separate one. They must merge.

Acceptance criteria :
- One `HarnessAuthority` type used across both crates (likely in cynic-protocol)
- Promotion rules are a pure function `can_promote(from: HarnessAuthority, to: HarnessAuthority) -> bool`
- Every mutation surface declares its minimum required authority
- No surface can operate with a weaker authority than it declares

### U3 — Admission Gate Integration

**Constitution ref :** §4 Budget Monotonicity, §5 Admission Gate, §6 Scope Hierarchy
**Depends on :** Audit D, U1

Target : Every mutation surface calls `admit_mutation` (or its evolved form) before mutating. No exceptions.

Acceptance criteria :
- builder_http.rs calls admission gate before accepting a build
- build_workload_submission_uc.rs calls admission gate before submitting
- release_webhook.rs calls admission gate before triggering source sync
- update_lifecycle calls admission gate before mutating host state
- A test that removes the admission gate from one surface and verifies the mutation is refused

### U4 — Proof Registry

**Constitution ref :** §8 Proof Obligations
**Depends on :** Audit C

Target : A canonical registry of proof schemas and a forward-compatible `ProofEnvelope` wrapper that any proof type can be wrapped into.

Acceptance criteria :
- Every proof type has a schema string constant
- Every proof type carries: schema, produced_at_unix_ms, producer_id, evidence_refs
- New proof fields can be added without breaking existing consumers (`#[serde(deny_unknown_fields)]` is NOT used; `#[serde(default)]` on new fields is)
- A proof produced in v1 can be read by a v2 consumer that doesn't know the new fields (forward-compat test)

### U5 — Time Injection

**Constitution ref :** §5 (budget expiry), §7 (attestation window)
**Depends on :** Audit E

Target : No call to `chrono::Utc::now()` outside of test code and the top-level runtime entrypoint. All time validation takes `now_unix_ms: u64` as an explicit parameter.

Acceptance criteria :
- `grep -r "Utc::now()" crates/` returns zero hits outside `tests/` and the runtime bootstrap
- Every TTL validation is pure given a clock parameter
- Every attestation expiry check is pure given a clock parameter
- A test that passes a past/future clock and verifies correct block/allow

### U6 — Rollback Intent Typing

**Constitution ref :** §5 Admission Gate (bounded action requires rollback intent)
**Depends on :** Audit B

Target : Replace all `&'static str` rollback descriptions with a typed `RollbackIntent` enum that carries the action, the target, and a proof reference.

Acceptance criteria :
- `RollbackIntent` enum in cynic-protocol with variants for every rollback-able action
- Every `HarnessScenario` carries `rollback: RollbackIntent`, not `rollback: HarnessRollback`
- The rollback action itself produces a `RollbackProof` as read-back evidence
- A test that triggers a rollback and verifies the `RollbackProof` is persisted

### U7 — Citizen Identity Completion

**Constitution ref :** §2 What Is a Citizen
**Depends on :** Audit D

Target : Evolve `AgentCitizenshipRecord` into `CitizenIdentity` that carries `CitizenClass`, harness binding, and admission evidence.

Acceptance criteria :
- `CitizenIdentity` struct exists and is serialize/deserializable
- `CitizenClass` enum: HumanAdmin, Machine, Agent, Scheduled
- Every active agent session has a `CitizenIdentity` in the claim store
- A session without a `CitizenIdentity` cannot mutate anything
- Round-trip JSON test with schema evolution (add a field, verify v1 JSON still parses)

---

## Phase 2 — Meta-Harness Construction

These items build the meta-harness itself. They depend on Phase 1 unifications.

### M1 — Family Report Unification

**Depends on :** U2, U4

Target : Each harness family (build, runtime, coordination scaling, coordination fault, agent loop) exposes a common `HarnessFamilyReport` shape so the meta-harness can aggregate them without matching on concrete types.

Acceptance criteria :
- A trait or generic function that any family implements to produce a `FamilyReport`
- `MetaHarnessReport` aggregates 5+ family reports without knowing their internals
- The weakest-authority-wins rule is a pure function with tests
- The most-severe-status-wins rule is a pure function with tests

### M2 — Simple Harness Surface

**Constitution ref :** industrial-grade observability without ceremony
**Depends on :** M1

Target : A single read-only surface that answers "is my CYNIC healthy?" for operators and agents.

Acceptance criteria :
- `simple_harness_check()` returns `SimpleHarnessReport` with status, kind, conformance_percent, critical_blockers, residual_unknowns
- Exposed as `cynic runtime harness --json --simple` (new flag, read-only)
- Never mutates; never fabricates; never hides unknowns
- A test that injects a critical blocker and verifies `SimpleHarnessReport.status == Blocked`

### M3 — Promotion Flow Enforcement

**Constitution ref :** §7 Harness Binding, AGENTS.md Runtime Harness progression
**Depends on :** M1, U2

Target : The harness promotion (lab → conformance → staging → canary → production) is not just documented but enforced by the admission gate.

Acceptance criteria :
- A citizen operating in `ClosedLocalLab` cannot submit a mutation targeting `Production`
- Promotion requires a `RuntimeHarnessTargetAttestation` from the current level before the next level admits
- A test that attempts cross-level mutation without proof and is refused
- A test that provides valid promotion proof and is admitted

---

## Phase 3 — Agent CLI & Sovereign Surface

These items expose the constitution to operators via CLI and MCP.

### S1 — Citizen CLI Surface

**Depends on :** U7, M2

Target : `cynic agent enroll/status/harness/mutate` surfaces for agent lifecycle.

Acceptance criteria :
- `cynic agent enroll --scope <scope> --budget <actions>:<wall_seconds>` creates a `CitizenIdentity` and persists it
- `cynic agent status --json` shows budgets consumed, claims held, harness, residual unknowns
- `cynic agent harness --json` delegates to `simple_harness_check()`
- `cynic agent mutate --scope <target> --budget <req>` calls admission gate before any mutation

### S2 — Constitution Amendment Surface

**Constitution ref :** §12 Amendment
**Depends on :** S1

Target : A formal path for amending the constitution with quorum and regression testing.

Acceptance criteria :
- A `constitution_amendment` schema type with: proposal_id, section, old_text, new_text, proposed_at, required_quorum
- An amendment is not admitted unless it passes `make check` (regression proof exists)
- An amendment violating the non-amplification invariant (§11) requires 2/3 quorum
- The amendment history is persisted and auditable

---

## Phase 4 — Deepening Audits (Post-Unification)

These audits happen after the unification is in place. They look for emergent violations that only appear at scale.

### DA1 — Amplification Detection Audit

Target : Run the runtime for N iterations and verify no citizen's effective authority grows beyond its grant.

Method : Instrument every budget mutation, every scope expansion, every authority escalation. Log any amplification event. Verify zero events after 10,000 scheduled mutations.

### DA2 — Proof Coverage Audit

Target : Every mutation path produces a proof that is consumed by a higher layer.

Method : Enumerate all mutation surfaces (from Audit D results). For each, verify the proof it produces is actually read by an observer or report layer. Flag orphan proofs (produced but never consumed) and blind mutations (mutating without producing any proof).

### DA3 — Federation Boundary Audit

Target : Verify that cross-node delegation always requires an explicit federation contract.

Method : Search for any path where a node's scheduler can mutate another node's state without going through the federation envelope (`cynic.agent_delegation_envelope.v1` for local, federation protocol for remote).

### DA4 — Temporal Consistency Audit

Target : Verify that all time windows (leases, attestations, promotions) use the same clock source and that clock skew between nodes cannot cause authority escalation.

Method : Model clock skew scenarios. Verify that a node with a fast clock cannot extend its lease beyond the parent's intent.

---

## Sequencing and Dependencies

```
Phase 0 (Audits) ──► Phase 1 (Unifications) ──► Phase 2 (Meta-Harness) ──► Phase 3 (CLI)
     │                      │                          │                       │
     ├── A Coordination     ├── U1 Claim Store         ├── M1 Family Report    ├── S1 CLI
     ├── B Authority        ├── U2 Authority Enum      ├── M2 Simple Surface   ├── S2 Amendment
     ├── C Proof            ├── U3 Admission Gate      └── M3 Promotion Flow
     ├── D Scope/Admission  ├── U4 Proof Registry
     └── E Budget/Time      ├── U5 Time Injection
                            ├── U6 Rollback Intent
                            └── U7 Citizen Identity
                                                        
Phase 4 (Deepening) runs continuously after Phase 2:
- DA1 after U1 + U3 + U7
- DA2 after U4 + M1
- DA3 after U1 + federation types
- DA4 after U5 + U1
```

## Acceptance Meta-Criteria

For the program to be considered complete:

1. Every audit (A–E) is published as a machine-readable report
2. Every unification (U1–U7) has a regression test that fails before the fix and passes after
3. Every meta-harness item (M1–M3) has a test with synthetic observations covering all four statuses (Converged, Degraded, Blocked, Unknown)
4. The CLI surfaces (S1–S2) have integration tests against a local lab
5. `make check` is green at every step — no wave lands red
6. The residual unknowns from Phase 0 audits are preserved and contribute to the final risk budget, never hidden

## What This Plan Does Not Cover

- Token economics, markets, or E-Score (product, not runtime constitution)
- Solana program audit runtime (a separate vertical)
- Inference backend selection and model routing (adapter concern, governed but not constitutional)
- Futard.io audience projection (narrative, not runtime authority)

---

*This plan is executable prose. Each phase produces evidence. The meta-acceptance is: every gap named in the audit has a corresponding unification that closes it, tested, with residual unknowns preserved.*
