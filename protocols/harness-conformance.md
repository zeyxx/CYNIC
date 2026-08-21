# CYNIC Meta-Harness — protocols/harness-conformance.md

## 1. Purpose

The **meta-harness** is the orchestrator that aggregates CYNIC's four existing
harness families into one deterministic conformance report. It is the
"harness of harnesses" — a bounded surface that never mutates unless
explicitly asked.

## 2. Harness Families (Existing)

| Family | Crate | Kind enum | Authority range |
|---|---|---|---|
| Build | `cynic-protocol::harness` | `HarnessScenarioId` (6 scenarios) | `ReadOnlyProjection` → `ExplicitCanary` |
| Runtime | `cynic-host::runtime_harness` | `RuntimeHarnessKind` (5 kinds) | `TestProjection` → `RuntimeAuthority` |
| Coordination Scaling | `cynic-protocol::coordination_scaling_harness` | scenarios at N=1,3,5 | quorum/standalone |
| Coordination Fault | `cynic-protocol::coordination_fault_harness` | fault injection (loss, release, stale-fence) | quorum + takeover |
| Agent Loop | `cynic-askesis::agent_loop` | 28 capability phases | `declared_projection_only` |

Each family is independently testable. The meta-harness composes them.

## 3. Meta-Harness Architecture

```rust
/// Canonical meta-harness report. Read-only unless `--apply` is set.
pub struct MetaHarnessReport {
    pub schema: &'static str,                // "cynic.meta_harness.report.v1"
    pub produced_at_unix_ms: u64,
    pub authority_level: HarnessAuthority,   // weakest authority across families
    pub overall_status: MetaHarnessStatus,   // converged | degraded | blocked | unknown
    pub families: Vec<HarnessFamilyReport>,  // one per family
    pub blockers: Vec<String>,               // all blockers, sorted + deduped
    pub residual_unknowns: Vec<String>,      // all unknowns, sorted + deduped
    pub can_mutate: bool,                    // true iff authority >= Staging && no blockers
}

pub enum MetaHarnessStatus {
    Converged,      // all families converged
    Degraded,       // some degraded but quorate
    Blocked,        // at least one blocker in any family
    Unknown,        // unresolved unknowns, no blockers
}

/// One family's contribution to the meta-report.
pub struct HarnessFamilyReport {
    pub family: HarnessFamily,
    pub status: HarnessStatus,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
}

pub enum HarnessFamily {
    Build,
    Runtime,
    CoordinationScaling,
    CoordinationFault,
    AgentLoop,
}
```

## 4. Execution Contract

```rust
pub fn reconcile_meta_harness(
    families: &[HarnessFamily],
    observations: &HarnessObservationSet,
    apply: bool,           // explicit --apply gate
) -> MetaHarnessReport;
```

**Deterministic.** Same inputs → same report. Every family contributes its
status; the meta-report takes the **weakest** authority and the **most severe**
status.

Resolution order (most severe wins):

```
Blocked > Unknown > Running > Planned > Completed
```

Mutation is authorized only when:
- `apply == true` (explicit operator intent)
- `overall_status != Blocked`
- `authority_level >= Staging` (lab/conformance cannot mutate production)
- All family `blockers` are empty
- At least one family has `status == Converged`

## 5. Harness Promotion Flow

Promotion moves a citizen through the runtime harness levels. Each step
requires **explicit proof** from the previous step.

```text
  closed_local_lab         conformance              staging
  [TestProjection]  ──►  [ConformanceGate]  ──►  [ReleaseCandidate]
       │                       │                       │
       │  lab_bundle           │  conformance_proof    │  staging_proof
       │  target_attestation   │  node_identity        │  release_candidate_proof
       │                       │                       │
       ▼                       ▼                       ▼
  ┌─────────┐  observe   ┌─────────────┐  observe   ┌──────────┐
  │ Lab     │ ────────►  │ Conformance │ ────────►  │ Staging  │
  │ Guest   │            │ Suite       │            │ Node     │
  └─────────┘            └─────────────┘            └──────────┘
                                                        │
                                                        │  canary_proof
                                                        ▼
                                                    ┌─────────┐  fleet_proof  ┌────────────┐
                                                    │ Canary  │ ────────────► │ Production │
                                                    └─────────┘               └────────────┘
                                                    [FleetCanary]             [RuntimeAuthority]
```

Proofs required at each transition:

| Transition | Proof |
|---|---|
| lab → conformance | `lab_bundle`, `target_attestation`, `guest_identity` |
| conformance → staging | `conformance_proof`, `node_identity`, `build_proof` |
| staging → canary | `staging_proof`, `release_candidate_proof` |
| canary → production | `fleet_proof`, `SLO_observation`, `rollback_intent_persisted` |

Each proof must be a `RuntimeHarnessTargetAttestation` (or equivalent) with
valid time window, verified identity, verified transport, and non-empty
`proof_refs`.

## 6. Simple Harness Surface

For operators and agents that want a minimal "is my CYNIC healthy?" surface:

```rust
/// Read-only, deterministic, no mutation. The simple harness surface.
pub fn simple_harness_check() -> SimpleHarnessReport;

pub struct SimpleHarnessReport {
    pub schema: &'static str,           // "cynic.simple_harness.v1"
    pub status: SimpleStatus,           // ok | degraded | blocked | unknown
    pub kind: Option<HarnessKind>,      // detected or configured kind
    pub conformance_percent: f64,       // 0.0 - 100.0, familled average
    pub critical_blockers: Vec<String>, // only the ones an operator must see
    pub residual_unknowns: Vec<String>,
}

pub enum SimpleStatus {
    Ok,         // all critical families converged
    Degraded,   // some non-critical families blocked
    Blocked,    // at least one critical blocker
    Unknown,    // unresolved unknowns
}
```

The simple surface **never** auto-mutates. It projects canonical decisions
from deeper layers; it never re-derives them.

### CLI surface (future, when transposed to Rust):

```bash
cynic runtime harness --json [--kind <kind>]          # existing
cynic runtime conformance --json [--profile <name>]   # existing
cynic agent harness --json                            # new: simple surface for agents
cynic agent status --json                             # new: citizen status
```

## 7. Constitution Binding

The meta-harness is the **executable projection** of the constitution
(`protocols/agent-constitution.md`). When the constitution says "budgets are
monotonic", the meta-harness produces a `HarnessCellReport` with status
`Blocked` and code `BudgetAmplificationDetected` if a surface ever violates
this.

## 8. Non-Goals

The meta-harness does **not**:
- Replace individual family harnesses — it composes them
- Mutate real state without explicit `--apply`
- Fabricate proof for families that have not observed
- Auto-promote across harness levels without read-back proof
- Hide unknowns to make the report look converged

## 9. Verification

When transposed to Rust, the meta-harness must be tested with:

- A synthetic fully-converged observation set → `Converged`
- A synthetic family with one blocker → `Blocked`
- A synthetic family with unresolved unknowns → `Unknown`
- A synthetic degraded-but-quorate scenario → `Degraded`
- A mutation attempt with `apply=true` but `status=Blocked` → refused
- A mutation attempt with `apply=false` regardless of status → refused

Every test locks the exact `blockers` and `unknowns` vectors — regression
proof that honest reporting survives refactors.

---

*Proposed location when transposed: `crates/cynic-protocol/src/meta_harness.rs`.*
*Composes: `harness.rs`, `runtime_harness.rs`, `coordination_scaling_harness.rs`,*
*`coordination_fault_harness.rs`, `agent_loop.rs`.*
