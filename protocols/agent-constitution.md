# CYNIC Agent Constitution — protocols/agent-constitution.md

## 1. Purpose and Authority

This document is the **normative constitution** for any agent that operates
within a CYNIC runtime. It is subordinate to `contracts/foundation.json` and
`AGENTS.md`. Where this document disagrees with executable Rust types in
`cynic-protocol`, the Rust types win until a migration lands.

This is prose — it becomes law only when expressed as Rust types in
`crates/cynic-protocol/src/agent_citizenship.rs`, tests, and schema contracts.
The pseudo-Rust in each section is intended to compile when transposed.

## 2. What Is a CYNIC Citizen

A **citizen** is any entity that may propose a bounded mutation within a CYNIC
runtime boundary. Citizens are typed, not trusted:

- `HumanAdmin` — a human principal holding machine enrollment credentials
- `MachineEnrollment` — a node holding an attested ed25519 keypair
- `AgentSession` — a registered agent loop (Hermes, Codex, Claude, CYNIC native)
- `ScheduledTask` — a cron-bound workload with a lease

Every citizen is identified by:

```rust
/// Canonical citizen identity. Portable across surfaces.
pub struct CitizenIdentity {
    pub schema: &'static str,               // "cynic.citizen_identity.v1"
    pub citizen_id: String,                 // unique within the organism
    pub citizen_class: CitizenClass,        // HumanAdmin | Machine | Agent | Scheduled
    pub scope: AgentScope,                  // Fleet | Node | Crate | Module | File
    pub mode: AgentMode,                    // Baseline | Cynic
    pub harness_kind: HarnessKind,          // the declared execution membrane
    pub budgets: AgentBudgets,              // monotonic, never self-amplifying
    pub residual_risk: Option<String>,      // outstanding, machine-readable
    pub evidence: Option<SecurityEvidenceState>, // read-back proof binding identity → scope
}
```

## 3. Scope Hierarchy

Scopes form a total order by **containment**, not equality:

```
Fleet ⊇ Node ⊇ Crate ⊇ Module ⊇ File
```

A citizen holding a `Crate` claim may mutate a `File` inside it, but **never**
a `Node` above it. Overlap is containment, not intersection — otherwise two
agents could mutate the same surface through differently-named scopes.

```rust
pub enum AgentScope {
    Fleet,   // specificity 0 — broadest
    Node,    // 1
    Crate,   // 2
    Module,  // 3
    File,    // 4 — narrowest
}

impl AgentScope {
    pub fn contains(&self, other: &Self) -> bool {
        self.specificity() <= other.specificity()
    }
}
```

## 4. Budget Monotonicity

Budgets are **monotonic downward**. A child citizen may receive a subset of
its parent's budgets, never an amplification. This is a hard invariant —
violation is a refusal, not a warning.

```rust
pub struct AgentBudgets {
    pub actions: u64,        // mutation budget — decremented per bounded action
    pub wall_seconds: u64,   // wall-time budget — decremented per session tick
}

pub fn budgets_within(parent: &AgentBudgets, child: &AgentBudgets) -> bool {
    child.actions <= parent.actions && child.wall_seconds <= parent.wall_seconds
}
```

**Rule:** amplification is always refused. A citizen that exhausts its budget
must request a new delegation from a parent — it may never mint its own.

## 5. Admission Gate

Every mutation must pass through the canonical gate. Surfaces **project**
this gate; they never re-derive it locally (AGENTS.md invariant #6).

```rust
pub fn admit_mutation(
    citizen: &CitizenIdentity,
    held_claim: Option<&AgentScope>,
    requested: &AgentBudgets,
    target_scope: &AgentScope,
    store_available: bool,
) -> MutationAdmission;

pub enum MutationAdmission {
    Admitted,
    Refused { codes: Vec<CynicErrorCode> },
}
```

Refusal reasons are **cumulative** — every violated contract surfaces at once.
Reporting only the first blocker makes the operator repair one and hit the next
blind.

Refusal codes:

| Code | Meaning |
|---|---|
| `AgentCoordinationStoreUnavailable` | store unreachable → explicit unknown, not permission |
| `AgentMutationWithoutClaim` | no claim held → no mutation, ever |
| `AgentScopeConflict` | held claim does not contain target |
| `AgentBudgetExceeded` | requested exceeds citizen's remaining budget |
| `HarnessKindMismatch` | harness authority too weak for the mutation class |
| `IsolationBoundaryUnverified` | operator host network safety not proved |

## 6. Mode Precedence

```
Cynic > Baseline
```

A `Cynic`-mode claim may **preempt** a `Baseline`-mode claim with the same
scope. Two claims in the same mode are **first-wins** by the claim store —
never preemption, otherwise two baseline agents would evict each other
indefinitely.

```rust
pub enum AgentMode { Baseline, Cynic }

impl AgentMode {
    pub fn preempts(self, other: Self) -> bool {
        matches!((self, other), (Self::Cynic, Self::Baseline))
    }
}
```

## 7. Harness Binding

Every citizen operates inside a declared harness. The harness is an execution
membrane — it is **not** the capability itself.

```rust
pub struct HarnessBinding {
    pub kind: HarnessKind,                  // ClosedLocalLab | Conformance | Staging | Canary | Production
    pub authority: HarnessAuthority,        // what mutations this kind may authorize
    pub isolation_boundary: &'static str,   // proven membrane description
    pub target_scope: &'static str,         // "single-host", "N nodes", "release candidate"
    pub mutates_host_network: bool,         // explicit flag — no silent mutation
}
```

Promotion contract (AGENTS.md § Runtime Harness):

```
ClosedLocalLab → Conformance → Staging → Canary → Production
```

Each step **must** produce a `RuntimeHarnessTargetAttestation` proving:
- target identity verified
- transport verified
- isolated network verified
- operator host network unchanged (for non-mutating steps)
- proof refs present and non-empty
- attestation window valid (observed_at < expires_at, not expired)

## 8. Proof Obligations

A successful command is **not** convergence. Every bounded action must produce
a machine-readable proof that a lower layer produced:

| Action | Proof type |
|---|---|
| Observe | `HarnessObservation`, `RuntimeHarnessTargetAttestation` |
| Claim/release | `LeaseVerification` |
| Build dispatch | `HarnessCellReport`, `BuildProof` |
| Runtime mutation | `RuntimeProofBundle`, `TunnelProof` |
| Agent task completion | `AgentLoopAuditReport` |

A proof that cannot be read back is **not** proof — it is a claim. The organism
prefers `Unknown` (honest) over `Complete` (fabricated).

## 9. Unknown Is Data

Residual unknowns are preserved as explicit `Vec<String>` in every report.
They contribute to residual risk — they never disappear through formatting.

Coverage values (from `contracts/foundation.json`):

```
partial | complete | unknown | blocked
```

`100% coverage` is not the target. Industrial-grade means reaching the target
thresholds with an explicit residual unknown budget.

## 10. Fractal Recursion

Every CYNIC node is a **sovereign recursion root**. A citizen may delegate to
child citizens within its own node using `cynic.agent_delegation_envelope.v1`.

Rules:
- A child stays on its parent's `node_id`
- A child names its parent
- Recursion depth increments exactly once per delegation
- Child budgets ≤ parent budgets (monotonic)
- Context is content-addressed evidence refs, never raw chat history

Cross-node delegation is **federation**, not local recursion — it requires an
explicit coordination contract (see `cynic-federation-pattern` skill).

## 11. Non-Amplification Invariant

No citizen, agent, model, market, or tool may silently amplify its own
authority. This is a structural invariant of the constitution:

- Budgets are monotonic downward
- Scope is containment, never expansion by claim
- Mode precedence is explicit (Cynic > Baseline), not emergent
- Proof is read-back, never claim-of-success
- Unknowns are preserved, never hidden

Any surface that introduces implicit amplification (a CLI that auto-grants
budgets, a scheduler that mints its own claims, an agent that rewrites its
own scope) is a constitutional violation and must be refused by the admission
gate.

## 12. Amendment

This constitution may be amended by:
1. Proposing a change to this document
2. Transposing the change into Rust types in `cynic-protocol`
3. Adding regression tests at the layer that owning the contract
4. Running `make check` green
5. Documenting the migration path for existing citizens

No amendment may violate the non-amplification invariant (§11) without a
two-thirds quorum of runtime authorities — and even then, the invariant is
preferred to be treated as immutable.

---

*This document is prose. Law lives in Rust. Transpose, compile, test.*
