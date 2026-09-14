# RFC-003: THE HERMETIC AGENT PARCEL PROTOCOL (ATOMIC SETTLEMENT & IMMUTABILITY)

- **Status**: Draft
- **Layer**: Protocol / Execution (`cynic-protocol`, `cynic-node`, `cynic-build`)
- **Authority**: Foundation §481 (SoC), Coherence Pattern, Proof-before-Mutation
- **Date**: 2026-09-14

---

## 1. Context & Motivation

Current agent workloads suffer from subtle race conditions and state mutability during transit:
1. **Plan Tampering in Flight**: Workload actions (`AgentBoundedAction`) had their paths or digests mutated between submission and execution, breaking cryptographic proofs (`plan_digest`).
2. **Lease vs Budget Window Drift**: Claims allowed a 60-second lease window while delegation budgets allowed 900 seconds, causing premature fencing token invalidation during compute-heavy compilation.
3. **Fragmented Validation**: Verification of lease tokens, completion evidence, and budget accounting was scattered across three disconnected layers.

Inspired by Bitcoin UTXO/HTLC models, this RFC establishes the **Hermetic Agent Parcel** as an indivisible, immutable, and single-spend execution unit.

---

## 2. Core Mathematical Model

### A. The Parcel Envelope (Immutable Intention)
An Agent Parcel is immutable once admitted:
$$\text{ParcelEnvelope} = \langle \text{WorkloadID}, \text{InputDigest}, \text{Action}, \text{Delegation}, \text{PlanDigest} \rangle$$
Where:
- $\text{Action} = (\text{argv}, \text{working\_directory}, \text{mutates}, \text{stop\_condition}, \text{rollback\_intent})$
- $\text{PlanDigest} = \text{SHA256}(\text{CanonicalAction})$ (Inviolable: transit layers MUST NOT alter any field).

### B. The Fenced Lease (Single-Worker Execution)
A worker obtains a time-locked lease:
$$\text{ParcelClaim} = \langle \text{LeaseID}, \text{WorkerNodeID}, \text{FencingToken}, T_{\text{granted}}, T_{\text{expires}}, T_{\text{max\_window}} \rangle$$
Invariants:
1. **Single-Spend**: $\forall t \in [T_{\text{granted}}, T_{\text{expires}}]$, no second claim may be issued on the same workload.
2. **Monotonic Fencing**: Worker actions are admitted iff $\text{FencingToken}_{\text{worker}} == \text{FencingToken}_{\text{latest}}$.
3. **Window Ceiling**: $T_{\text{expires}} \le T_{\text{granted}} + T_{\text{max\_window}}$, where $T_{\text{max\_window}} \ge \text{Delegation.Budget.WallTime}$.

### C. The Settlement Function (Atomic Verification)
The settlement is a pure Zero-I/O function:
$$\text{SettleParcel}(\text{Record}, \text{Claim}, \text{Completion}, T_{\text{now}}) \to \text{Result}\langle \text{SettledParcelReceipt}, \text{ParcelViolation} \rangle$$

The transition to `Completed` succeeds IF AND ONLY IF:
1. $\text{Claim.Guard} == \text{Worker.Guard} \land \text{Claim.FencingToken} == \text{Record.LastFencingToken}$
2. $T_{\text{now}} \le \text{Claim.ExpiresAt}$
3. $\text{Completion.ConsumedBudget.Actions} \le \text{Delegation.Budget.Actions}$
4. $\text{Completion.ConsumedBudget.WallTime} \le \text{Delegation.Budget.WallTime}$
5. $\forall \text{proof} \in \text{Delegation.ExpectedProofs}, \exists \text{evidence} \in \text{Completion.EvidenceRefs} \text{ s.t. } \text{evidence.kind} == \text{proof}$

---

## 3. Pure Domain Transposition (`cynic-protocol`)

The settlement function lives purely in `cynic-protocol::agent_workload::settlement`:

```rust
pub struct SettledParcelReceipt {
    pub schema: &'static str,
    pub workload_id: String,
    pub settled_at_ms: u64,
    pub status: AgentCompletionStatus,
    pub final_record_digest: String,
    pub evidence_count: usize,
}

pub fn settle_parcel(
    record: &AgentWorkloadRecord,
    guard: &LeaseGuard,
    completion: &AgentCompletionEnvelope,
    now_ms: u64,
) -> Result<AgentWorkloadRecord, AgentWorkloadViolation>;
```

---

## 4. Architectural Boundary Invariants

1. **Zero-I/O**: `settle_parcel` takes pure data structures and returns pure data structures. No network, no disk, no syscalls.
2. **No Upward Dependency**: `cynic-protocol` never imports `cynic-node`, `cynic-build`, or `tokio`.
3. **Fail-Closed**: Any missing evidence, expired lease, or mutated working directory results in an explicit terminal violation, never a silent recovery.
