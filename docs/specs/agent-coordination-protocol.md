# CYNIC Agent Coordination Protocol — protocols/agent-coordination-protocol.md

## Purpose

This document specifies the **executable** protocol for any agent that operates
within a CYNIC runtime. It supersedes the prose-based agent coordination section
in AGENTS.md with a machine-readable contract that any agent implementation
(Hermes, Codex, Claude, native CYNIC) can wire to.

**Authority:** This protocol is normative. AGENTS.md § Agent Coordination
Claims is now a projection of this document. Where they disagree, this wins.

**Design principle:** The protocol is a Rust state machine first. MCP tools,
CLI subcommands, and JSON files are projections. The Rust types and their
tests are the single source of truth.

## Scope

| In scope | Out of scope |
|---|---|
| Agent identification and session lifecycle | Model selection and inference routing |
| Scope claim, heartbeat, release | Build scheduler internals (see `cynic.agent_workload_envelope.v1`) |
| Budget monotonicity enforcement | Fleet formation protocols |
| Mutation admission gate | Solana program settlement |
| Cross-agent conflict detection | Futard.io audience projection |

## State Machine

A CYNIC agent session moves through exactly these states:

```
UNINITED → IDENTIFIED → REGISTERED → CLAIMED → HEARTBEATING → MUTATING → RELEASED
                                       ↓                       ↓
                                    BLOCKED                 BLOCKED
```

**Transitions are atomic and verified.** Each transition requires a precondition
to hold (a pure Rust function). A session may never skip a state.

### State definitions

```
UNINITED    — Session created, no agent identity declared.
IDENTIFIED  — Agent has declared identity (agent_id, credentials).
REGISTERED  — Kernel acknowledged identity and issued session token.
CLAIMED     — Agent holds at least one active claim on a scope.
HEARTBEATING— Claim TTL is being renewed periodically.
MUTATING    — Agent is performing a bounded mutation inside its claimed scope.
RELEASED    — All claims released, session closed cleanly.
BLOCKED     — Transition refused (precondition violated). Must surface to operator.
```

## Protocol Phases

### Phase 1 — IDENTIFY

**Purpose:** Prove who the agent is.

**Precondition:** None (entry point).

**Postcondition:** `agent_id` is declared and bound to an attested identity.

**MCP tool:** `cynic_agent_identify` (or env-based for local).

**Rust type:**
```rust
pub struct AgentIdentity {
    pub schema: &'static str,               // "cynic.agent_identity.v1"
    pub agent_id: String,                   // unique within organism
    pub agent_kind: AgentKind,              // Hermes, Codex, Claude, CynicNative, HumanAdmin
    pub credentials: CredentialAttestation, // not the credential itself
    pub attested_at_unix_ms: u64,
    pub evidence: Option<SecurityEvidenceState>,
}

pub enum AgentKind {
    Hermes,
    Codes,
    Claude,
    CynicNative,
    HumanAdmin,
}
```

**Validation gate:**
```rust
pub fn validate_agent_identity(identity: &AgentIdentity) -> Result<(), Vec<String>>
```

**Errors:**
- `agent_id_empty` — must be non-empty
- `attestation_missing` — credentials not proven
- `attestation_expired` — evidence past validity window

### Phase 2 — REGISTER

**Purpose:** Announce the agent to the kernel coordination system.

**Precondition:** `IDENTIFIED` — agent identity valid.

**Postcondition:** Kernel issued a session token and the agent is known.

**MCP tool:** `cynic_coord_register`.

**Rust type:**
```rust
pub struct RegistrationRecord {
    pub schema: &'static str,              // "cynic.registration.v1"
    pub agent_id: String,
    pub session_id: String,
    pub registered_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub capabilities: Vec<String>,         // tools the agent may call
}
```

**Validation gate:**
```rust
pub fn validate_registration(reg: &RegistrationRecord) -> Result<(), Vec<String>>
```

**Kernel endpoint:** `POST /coord/register`.

### Phase 3 — CLAIM

**Purpose:** Acquire exclusive (or scoped) access to a resource before mutation.

**Precondition:** `REGISTERED` — session active.

**Postcondition:** Agent holds a claim with a valid lease and fencing token.

**MCP tool:** `cynic_coord_claim` / `cynic_coord_claim_batch`.

**Rust type:**
```rust
pub struct ClaimRequest {
    pub schema: &'static str,              // "cynic.claim_request.v1"
    pub agent_id: String,
    pub session_id: String,
    pub scope: ClaimScope,
    pub claim_type: ClaimType,             // Exclusive | SharedObserve
    pub budget: AgentBudgets,
    pub ttl_seconds: u64,
    pub parent_claim_id: Option<String>,
}

pub struct Claim {
    pub schema: &'static str,              // "cynic.claim.v1"
    pub claim_id: String,
    pub request: ClaimRequest,
    pub lease_id: String,
    pub fencing_token: u64,
    pub granted_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub max_lease_window_ms: u64,          // hard cap from grant time
    pub status: ClaimStatus,
    pub preempted_by: Option<String>,
}

pub enum ClaimStatus {
    Active,
    Expired,
    Released,
    Preempted,
    Blocked,
}
```

**Validation gate:**
```rust
pub fn validate_claim_request(req: &ClaimRequest) -> Result<(), Vec<String>>
```

**Conflict detection:**
```rust
pub fn claim_conflicts(existing: &[Claim], incoming: &ClaimRequest) -> Option<ClaimConflict>
```

**Rules:**
1. A claim whose scope contains another agent's active claim → conflict.
2. Same mode, same scope → first-wins. Second waits or picks another scope.
3. Cynic-mode preempts baseline-mode. Baseline gets grace period.
4. Parent budget >= sum of children budgets (monotonic).
5. TTL may not exceed `max_lease_window_ms`.

**Kernel endpoint:** `POST /coord/claim`.

### Phase 4 — HEARTBEAT

**Purpose:** Renew claim TTL while the agent is alive and working.

**Precondition:** `CLAIMED` — at least one active claim.

**Postcondition:** `expires_at_unix_ms` extended. Fencing token unchanged.

**MCP tool:** `cynic_coord_renew`.

**Rust type:**
```rust
pub struct HeartbeatRequest {
    pub schema: &'static str,              // "cynic.heartbeat_request.v1"
    pub agent_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
}
```

**Validation gate:**
```rust
pub fn validate_heartbeat(req: &HeartbeatRequest, active: &Claim) -> Result<(), Vec<String>>
```

**Rules:**
1. Heartbeat must arrive before `expires_at_unix_ms`.
2. Fencing token must match.
3. Renewed expiry must not exceed `granted_at_ms + max_lease_window_ms`.

**Kernel endpoint:** `POST /coord/renew`.

### Phase 5 — MUTATE (gated)

**Purpose:** Perform a bounded mutation within the claimed scope.

**Precondition:** `CLAIMED` + active claim + admission gate passed.

**Postcondition:** Mutation applied, proof generated.

**MCP tool:** Not a separate tool — the mutation itself (e.g. `cynic_validate`,
`cynic_source_control`) calls the admission gate internally.

**Rust type:**
```rust
pub struct MutationRequest {
    pub schema: &'static str,              // "cynic.mutation_request.v1"
    pub agent_id: String,
    pub claim_id: String,
    pub target_scope: ClaimScope,
    pub action: AgentBoundedAction,
    pub budget_delta: AgentBudgets,        // consumed by this mutation
    pub rollback_intent: String,
}
```

**Validation gate:**
```rust
pub fn admit_mutation(
    claim: &Claim,
    request: &MutationRequest,
    store_available: bool,
) -> MutationAdmission;
```

**Rules:**
1. Claim must be active (not expired, not preempted).
2. Target scope must be contained in the claim's scope.
3. Budget delta must not exceed remaining budget.
4. If `store_available == false`, mutation is refused with
   `AgentCoordinationStoreUnavailable`.

### Phase 6 — RELEASE

**Purpose:** Cleanly close the session and free all claims.

**Precondition:** Any state after `REGISTERED`.

**Postcondition:** All claims released, session closed.

**MCP tool:** `cynic_coord_release`.

**Rust type:**
```rust
pub struct ReleaseRequest {
    pub schema: &'static str,              // "cynic.release_request.v1"
    pub agent_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub proof_refs: Vec<String>,           // evidence of completed work
}
```

**Kernel endpoint:** `POST /coord/release`.

## Canonical Rust Location

```
crates/cynic-protocol/src/agent_coordination.rs
```

All pure predicates live in:
```
crates/cynic-protocol/src/agent_coordination/predicates.rs
```

MCP tools project these types — they never re-derive policy.

## MCP Tool Inventory

| Phase | Tool name | Mutates | Auth required |
|---|---|---|---|
| IDENTIFY | `cynic_agent_identify` | No | No |
| REGISTER | `cynic_coord_register` | Yes | Yes |
| CLAIM | `cynic_coord_claim` | Yes | Yes |
| CLAIM (batch) | `cynic_coord_claim_batch` | Yes | Yes |
| HEARTBEAT | `cynic_coord_renew` | Yes | Yes |
| RELEASE | `cynic_coord_release` | Yes | Yes |
| STATUS | `cynic_coord_who` | No | No |

## Failure Modes

| Situation | Behavior |
|---|---|
| Kernel unreachable during CLAIM | Refuse: `blocked_disconnected` |
| Heartbeat fails repeatedly | Claim expires, agent must re-claim or stop mutating |
| Conflicting claim from another agent | Second agent waits or picks another scope |
| Cynic-mode preempts baseline | Baseline agent gets grace period (configurable, default 300s) |
| Budget exhausted | All further mutations refused until new claim |
| Session ends without release | Claims expire via TTL; stale heartbeat detection cleans up |

## Implementation Checklist

1. [ ] Rust types + validation gates in `cynic-protocol`
2. [ ] Pure predicates in `cynic-protocol::agent_coordination::predicates`
3. [ ] Unit tests for each transition
4. [ ] MCP tool wiring in `cynic-mcp`
5. [ ] Kernel REST handlers in `cynic-kernel`
6. [ ] CLI subcommands in `cynic-cli`
7. [ ] Hermes background heartbeat (background task or cron)
8. [ ] AGENTS.md updated to reference this protocol (not duplicate it)

---

*This protocol is executable prose. Law lives in Rust. Transpose, compile, test.*
