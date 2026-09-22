# Cynic Agent Bus — Architecture Post-Revision

> **Trace**: 4-prism review (446 tool calls, 41 findings, 7 blockers, 9 major, 5 minor) → consolidation → architecture decision.
> **Date**: 2026-09-22
> **Phase**: 0 — Constitutional grounding for Phase 4 rewrite

---

## 1. Constitutional Foundation

### 1.1 Separation of Powers (reaffirmed from foundation.json)

The Agent Bus is the **Executive** branch. It executes admitted workloads. It does not make law (Legislative = Constitutional Court), interpret law (Judicial = Auditor), or constitute power (Constituent = Human).

| Power | Agent | Bus responsibility |
|-------|-------|--------------------|
| Constituent | Human | Defines `Budget`, `Policy` |
| Legislative | Constitutional Court | Issues `CourtMandate` rules |
| Executive | Agent Bus | Executes `Task`s, acquires `Claim`s |
| Judicial | Auditor | Verifies `Claim`s + `Proof`s |
| Control | Architecture tests | Read-back proofs, boundary enforcement |

### 1.2 Domain Invariant (Layer 0 — immutable)

```
A Task transitions to Completed
  if and only if
its Claim is Released
  AND
a Proof is appended to the Ledger.
```

This invariant lives in the domain, not in the adapters. It is tested by `architecture_boundary.rs`.

### 1.3 What belongs where

| Constitutional (Layer 0 — bus domain types) | Evolutive (Layer 1 — policy/config) |
|---|---|
| `AgentId` derivation (ed25519 + BLAKE3) | TTL durations (1800s default) |
| `Hash32` / `Proof` structure | Scope overlap rules |
| `Claim` existence | Budget denominations (actions, wall_seconds) |
| `Scope` shape | Guardrail defaults (stagnation_window, max_turns) |
| `Task` identity | Agent alternation pattern |

---

## 2. Crate Topology (post-revision)

```
cynic-protocol              ← Layer 0: pure types, ports, policy contracts
    ↑ depends on
cynic-agent-bus             ← Layer 1: domain pure (zéro IO/HTTP)
    ↑ depends on
cynic-agent-bus-server      ← Layer 2: REST projection (Axum, tokio::net)
    ↑ depends on
cynic-agent-bus-coordinator ← Layer 3: LLM-driven orchestration (spawns agents)
```

**Rule**: dependencies point DOWNWARD only. `cynic-agent-bus` never imports `cynic-agent-bus-server`. `cynic-agent-bus-coordinator` depends on the bus via `cynic-agent-bus` trait impls.

---

## 3. Hexagonal Architecture (enforced)

### 3.1 Domain (pure, zero IO)

```
src/domain/
    mod.rs          ← module declarations only
    agent_id.rs     ← AgentId, AgentIdError
    task.rs         ← Task, TaskId, TaskKind (trimmed)
    claim.rs        ← Claim, ClaimId, ClaimStatus, ClaimError
    scope.rs        ← Scope, ScopeKind, scope_overlap (pure function)
    budget.rs       ← Budget, ComputeCredits, TimeUnit
    proof.rs        ← Proof, ProofHash, verify_proof (pure)
```

**Constraints** (tested by `architecture_boundary.rs`):
- No `use std::`
- No `use tokio`
- No `use reqwest`
- No `use crate::infra::`
- No `use crate::adapters::`

### 3.2 Ports (traits only)

```
src/ports/
    mod.rs
    claim_store.rs  ← ClaimStore trait
    ledger.rs       ← Ledger trait
    executor.rs     ← AgentExecutor trait (replaces run_turn_stub)
    identity.rs     ← IdentityProvider trait
```

**Constraints**: no `impl Foo for Bar` in ports. No concrete types. Only trait + associated types.

### 3.3 Adapters (implement ports)

```
src/adapters/
    mod.rs
    in_memory_claim_store.rs
    in_memory_ledger.rs
    keypair_identity.rs
```

**Constraints**: adapters depend on domain types, never on infra. Adapters can use `tokio`, `std::sync`, `Arc<Mutex<>>`.

---

## 4. Agent Taxonomy (trimmed to used variants)

Post-minimalist review: collapse to 3 variants, expand when behavior differs.

```
Solo     — one agent, one file/crate
Chain    — sequential handoff (parent → child)
Fanout   — parallel dispatch (N workers → 1 merge)
```

`MultiPrism` is a **quality gate**, not a task kind. `CrossCrate` is a **scope**, not a kind. `Organic` is a **routing strategy**, not a kind.

---

## 5. Claim Lifecycle (corrected)

```
         acquire
  Queued ──────→ Active
                  │
                  │ expire (TTL) ──→ Expired
                  │
                  │ release ───────→ Released
                  │
                  │ preemption ────→ Preempted
                  │
                  │ budget exhaust ─→ Blocked
```

### 5.1 Invariants

- **Claim existence implies scope lock**: if a claim is `Active`, no overlapping claim can be acquired.
- **TTL implies auto-expiry**: if `now > acquired_at + ttl_seconds`, claim transitions to `Expired`.
- **Budget exhaust implies block**: if `consumed_actions >= actions` OR `elapsed >= wall_seconds`, claim transitions to `Blocked`.
- **Release requires proof**: `release()` requires a `Proof` input.

### 5.2 Anti-vectors (from adversarial review)

| Vector | Mitigation |
|--------|-----------|
| Cosmetic variance beats stagnation | Semantic diff threshold (token overlap > 0.8) |
| `wall_seconds` ignored | `check_guardrails` reads wall-clock elapsed |
| Wildcard `agent_id = "*"` release | Removed — exact agent_id required |
| Ghost claims (expired but held) | TTL checker task + `is_expired()` predicate |
| Proof signature unverified | `Proof` has no signature field — only content hash. Signature added when `ed25519` verification is real |

---

## 6. Task → Claim → Proof invariant

```
┌────────────────────────────────────────────────────────────────┐
│                                                                │
│  Task.Created ──acquire──→ Claim.Active                       │
│       │                         │                              │
│       │                   execute (N turns)                    │
│       │                         │                              │
│       │                   Proof.finalize()                     │
│       │                         │                              │
│  Task.Completed ←──release── Claim.Released + Proof appended  │
│                                                                │
│  ANY divergence from this invariant = system halt              │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

**Enforcement**: `ClaimManager::release()` requires a `Proof`. `Ledger::complete_task()` requires a `Proof`. Either both happen (atomic) or neither does.

---

## 7. Orchestrator Design (replaces stub)

### 7.1 Span of Control (military pyramid)

```
Human
└── Coordinator (1 per fleet)
    ├── Lead_Forensics
    ├── Lead_Kernel
    ├── Lead_Inference
    ├── Lead_Security
    ├── Lead_Protocol
    └── Lead_MCP

Each Lead manages 3-7 Workers.
```

**Rule**: `N_direct_reports ∈ [3, 7]` (Miller's law). Exceeding 7 triggers escalation.

### 7.2 Turn Loop (corrected)

```rust
async fn run_turn(task: &Task, agent: &AgentExecutor) -> Result<TurnResult, TurnError> {
    // 1. Check claim is active
    // 2. Check budget remaining (both actions AND wall_seconds)
    // 3. Execute agent call
    // 4. Update stagnation window with semantic hash
    // 5. Append to ledger
    // 6. Increment consumed_actions
    // 7. Return result
}
```

### 7.3 Stagnation Detection (semantic, not cosmetic)

```rust
fn detect_stagnation(outputs: &[String], window: usize, threshold: f64) -> bool {
    if outputs.len() < window { return false; }
    let recent = &outputs[outputs.len() - window..];
    // Token overlap ratio (Jaccard) — threshold 0.8
    let all_high_overlap = recent.windows(2).all(|w| jaccard(&w[0], &w[1]) > threshold);
    all_high_overlap
}
```

### 7.4 AgentExecutor Port

```rust
#[async_trait]
pub trait AgentExecutor: Send + Sync {
    /// Execute one turn for the given task. Returns the agent's output.
    async fn execute_turn(&self, task: &Task, turn_input: &str) -> Result<String, AgentError>;
    
    /// Spawn a new agent session (idempotent — returns existing session if valid).
    async fn spawn(&self, agent_kind: AgentKind) -> Result<SessionId, AgentError>;
    
    /// Terminate an agent session (graceful → force after timeout).
    async fn terminate(&self, session: SessionId) -> Result<(), AgentError>;
    
    /// Check if an agent session is alive (heartbeat).
    async fn is_alive(&self, session: SessionId) -> bool;
}
```

---

## 8. Scope Model (corrected)

### 8.1 Scope kinds and overlap

```
Crate { path: "crates/foo" }
    └── contains → Module { path: "crates/foo/src/bar" }
                      └── contains → File { path: "crates/foo/src/bar/baz.rs" }
```

### 8.2 Overlap function (pure, deterministic)

```rust
fn scope_overlap(existing: &Scope, new: &Scope) -> bool {
    match (existing, new) {
        (File(a), File(b)) => a == b,
        (Module(a), Module(b)) => a == b,
        (Crate(a), Crate(b)) => a == b,
        (Crate(a), _) => new.path().starts_with(a),
        (_, Crate(b)) => existing.path().starts_with(b),
        (Module(a), File(b)) => b.starts_with(a),
        (File(a), Module(b)) => a.starts_with(b),
    }
}
```

### 8.3 Canonical predicate alignment

`scope_overlap` in bus MUST match `cynic-protocol::policy::scopes_overlap`. If they diverge, the bus predicate is wrong (canonical is source of truth).

---

## 9. Anti-Patterns (from review)

| Anti-pattern | Status | Action |
|---|---|---|
| `run_turn_stub` | ❌ Delete | Replace with `AgentExecutor` trait |
| `Signature` field in `Proof` | ❌ Delete | Re-add only with real `ed25519` verification |
| `orchestrator.rs` hardcoded alternation | ❌ Delete | Replace with `Coordinator` + `AgentExecutor` |
| `TaskKind` 7 variants | ❌ Trim | Collapse to `Solo`, `Chain`, `Fanout` |
| `Budget::wall_seconds` dead | ❌ Fix | Add wall-clock check in `check_guardrails` |
| `ScopeKind::File` hardcoded for all | ❌ Fix | Map `TaskKind` → `ScopeKind` |
| `ClaimManager` direct `InMemoryLedger` dep | ❌ Remove | Inject via `Ledger` trait |
| Duplicate crate nesting | ❌ Delete | `rm -rf crates/cynic-agent-bus/crates/` |
| `cynic-protocol` dep unused | ❌ Remove or use | Either import types or remove dep |

---

## 10. Proof Architecture

### 10.1 Content-addressed proofs (current, valid)

```rust
pub struct Proof {
    pub parent_task_id: TaskId,
    pub child_task_id: TaskId,
    pub output_hash: Hash32,        // SHA-256 of final output
    pub scope: Scope,
    pub consumed_budget: Budget,
    pub timestamp_unix_s: u64,
}
```

No `Signature` field. Signatures are added when:
1. `ed25519-dalek` keys are wired to `AgentExecutor` sessions.
2. `Proof::verify(ed25519_pubkey)` is implemented and tested.

### 10.2 Handoff proof (Task → Task)

```rust
pub struct HandoffProof {
    pub from_task: TaskId,
    pub to_task: TaskId,
    pub diff_hash: Hash32,           // SHA-256 of (input ⊕ output diff)
    pub from_agent: AgentId,
    pub to_agent: AgentId,
    pub scope: Scope,
    pub timestamp_unix_s: u64,
}
```

---

## 11. Server Crate (projection, separate)

### 11.1 Responsibilities

- HTTP server (Axum or actix-web)
- Request/response serialization (JSON)
- Authentication (API keys / mTLS)
- Rate limiting

### 11.2 Endpoints

```
GET    /health                    → health check
POST   /tasks                     → create task
GET    /tasks/:id                 → get task status
POST   /claims                    → acquire claim
DELETE /claims/:id                → release claim
POST   /collaborate               → start multi-turn collaboration
GET    /ledger/:task_id           → get task ledger
```

### 11.3 Dependencies

`cynic-agent-bus` (domain), `axum`, `serde_json`, `tokio::net`, `tower-http`.

---

## 12. Implementation Sequence

| Phase | Deliverable | Acceptance |
|-------|-------------|------------|
| 0.1 | Delete nested duplicate crate | `cargo check -p cynic-agent-bus` passes |
| 0.2 | Trim `TaskKind` to 3 variants | No `match` on unused variants |
| 0.3 | Map `TaskKind` → `ScopeKind` correctly | `Crate` task → `ScopeKind::Crate` |
| 0.4 | `Ledger` trait with 3 methods + 2 events | Tests pass |
| 0.5 | `ClaimManager::release()` requires `Proof` | Cannot release without proof |
| 0.6 | `check_guardrails` reads wall-clock | Wall budget enforced |
| 0.7 | Remove `Signature` from `Proof` | No `ed25519` dependency unless verified |
| 0.8 | `AgentExecutor` trait defined | Port layer exists, no impl |
| 0.9 | Architecture boundary tests updated | Tests enforce domain purity |
| 0.10 | `cynic-agent-bus-server` scaffold | Axum server compiles |
| 0.11 | `Coordinator` scaffold | Span of control 3-7 enforced |

---

## 13. What was cut (and why)

| Cut | Reason |
|-----|--------|
| `Signature` in `Proof` | Theater of security — not verified |
| 4 of 7 `TaskKind` variants | No behavior differs — pure speculation |
| `orchestrator.rs` entire | Stub masquerading as architecture |
| `parent_task_id` cascade | No referential integrity — silent orphans |
| `result`/`error` simultaneous nullable | Invariant untested |
| Nested duplicate crate | Build broken, drift accelerator |
| `ed25519` in bus (for now) | Signature not verified — remove until wired |

---

*End of document. 41 findings from 4-prism review consolidated. Awaiting approval to begin Phase 0.1.*
