# CYNIC Agent Protocol — Complete Specification

## Purpose

This is the **complete executable specification** for the CYNIC agent protocol.
It unifies constitution, Git coordination, lifecycle management, proofs,
conflict resolution, and store synchronization into one coherent document.

**Authority:** This document supersedes all previous partial specifications.
Where sub-documents disagree, this document wins.

**Related documents:**
- `protocols/agent-constitution.md` — constitutional invariants
- `protocols/agent-git-protocol.md` — Git branching and merge protocol

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Agent Session                                │
│                                                                      │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐        │
│  │IDENTIFY  │→  │REGISTER  │→  │CLAIM     │→  │HEARTBEAT │        │
│  │          │   │          │   │          │   │          │        │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘        │
│       │              │              │              │                │
│       ▼              ▼              ▼              ▼                │
│  ┌─────────────────────────────────────────────────────────┐       │
│  │              Mutation Gated by Active Claim              │       │
│  │  - file patch/write     (File scope)                     │       │
│  │  - cargo check/test     (Crate scope)                    │       │
│  │  - git commit/push      (any scope with claim)           │       │
│  └─────────────────────────────────────────────────────────┘       │
│       │              │              │                               │
│       ▼              ▼              ▼                               │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐                      │
│  │PROVE     │   │CONFLICT  │   │RELEASE   │                      │
│  │          │   │          │   │          │                      │
│  └──────────┘   └──────────┘   └──────────┘                      │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ MCP / CLI / REST
┌─────────────────────────────────────────────────────────────────────┐
│                     CYNIC Kernel (SurrealDB)                        │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐            │
│  │ agent_claims│  │ coord_lease  │  │ proofs         │            │
│  │ (durable)   │  │ (fencing)    │  │ (evidence)     │            │
│  └─────────────┘  └──────────────┘  └────────────────┘            │
└─────────────────────────────────────────────────────────────────────┘
```

## Part 1 — Lifecycle States

### State machine

```
UNINITED → IDENTIFIED → REGISTERED → CLAIMED → HEARTBEATING → MUTATING → RELEASED
                          │              │           │            │
                          │              │           │            │
                          ▼              ▼           ▼            ▼
                       FAILED        EXPIRED     PREempted    PROVEN
```

### State definitions

| State | Meaning | Allowed transitions |
|---|---|---|
| `UNINITED` | Session created, no identity | `IDENTIFIED` |
| `IDENTIFIED` | Identity declared and validated | `REGISTERED`, `FAILED` |
| `REGISTERED` | Kernel issued session token | `CLAIMED`, `FAILED` |
| `CLAIMED` | Holds at least one active claim | `HEARTBEATING`, `EXPIRED` |
| `HEARTBEATING` | Claim TTL being renewed | `MUTATING`, `EXPIRED`, `PREEMPTED` |
| `MUTATING` | Performing bounded mutation | `HEARTBEATING`, `RELEASED` |
| `RELEASED` | All claims released cleanly | `CLAIMED` (new session) |
| `EXPIRED` | TTL lapsed, no heartbeat | `RELEASED` |
| `PREEMPTED` | Another agent preempted | `RELEASED` |
| `FAILED` | Unrecoverable error | `RELEASED` |

## Part 2 — Types and Canonical Structures

### AgentIdentity

```rust
pub struct AgentIdentity {
    pub schema: String,                    // "cynic.agent_identity.v1"
    pub agent_id: String,                  // unique within organism
    pub agent_kind: AgentKind,             // Hermes | Codex | Claude | CynicNative | HumanAdmin
    pub credentials: CredentialAttestation,
    pub attested_at_unix_ms: u64,
    pub evidence: Option<SecurityEvidenceState>,
}
```

### ClaimRequest → Claim → ClaimProof

```rust
pub struct ClaimRequest {
    pub schema: String,                    // "cynic.claim_request.v1"
    pub agent_id: String,
    pub session_id: String,
    pub scope: ClaimScope,
    pub claim_type: ClaimType,             // Exclusive | SharedObserve
    pub budget: AgentBudgets,
    pub ttl_seconds: u64,
    pub max_lease_window_ms: u64,
    pub parent_claim_id: Option<String>,
}

pub struct Claim {
    pub schema: String,                    // "cynic.claim.v1"
    pub claim_id: String,
    pub request: ClaimRequest,
    pub lease_id: String,
    pub fencing_token: u64,
    pub granted_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub max_lease_window_ms: u64,
    pub status: ClaimStatus,               // Active | Expired | Released | Preempted | Blocked
    pub preempted_by: Option<String>,
}

pub struct ClaimProof {
    pub schema: String,                    // "cynic.claim_proof.v1"
    pub claim_id: String,
    pub evidence_refs: Vec<String>,
    pub proven_at_unix_ms: u64,
}
```

### HeartbeatRequest → HeartbeatProof

```rust
pub struct HeartbeatRequest {
    pub schema: String,                    // "cynic.heartbeat_request.v1"
    pub agent_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
}

pub struct HeartbeatProof {
    pub schema: String,                    // "cynic.heartbeat_proof.v1"
    pub lease_id: String,
    pub previous_expiry_ms: u64,
    pub new_expiry_ms: u64,
    pub beats_at_unix_ms: u64,
}
```

### ReleaseRequest → ReleaseProof

```rust
pub struct ReleaseRequest {
    pub schema: String,                    // "cynic.release_request.v1"
    pub agent_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub proof_refs: Vec<String>,           // evidence of completed work
}

pub struct ReleaseProof {
    pub schema: String,                    // "cynic.release_proof.v1"
    pub lease_id: String,
    pub released_at_unix_ms: u64,
    pub proof_refs: Vec<String>,
    pub final_state_digest: String,
}
```

### ClaimScope (hierarchy)

```rust
pub enum ClaimScope {
    Fleet,
    Node { node_id: String },
    Crate { crate_name: String },
    Module { crate_name: String, module_path: String },
    File { path: String },
}

impl ClaimScope {
    pub fn contains(&self, other: &Self) -> bool { /* ... */ }
    pub fn specificity(&self) -> u8 { /* Fleet=0, File=4 */ }
}
```

## Part 3 — Pure Predicates

All decision logic lives in `cynic_protocol::agent_coordination::predicates`.
No surface may re-derive these locally (invariant #6).

```rust
/// Whether two claims have overlapping scopes.
pub fn scopes_overlap(a: &ClaimScope, b: &ClaimScope) -> bool;

/// Whether a claim is expired at the given timestamp.
pub fn is_expired(claim: &Claim, now_unix_ms: u64) -> bool;

/// Whether a claim's heartbeat is stale (no renewal within 80% of TTL).
pub fn is_heartbeat_stale(claim: &Claim, now_unix_ms: u64) -> bool;

/// Whether an incoming claim conflicts with an existing active claim.
pub fn claim_conflicts(existing: &Claim, incoming: &ClaimRequest) -> Option<ClaimConflict>;

/// Whether one claim can preempt another.
pub fn can_preempt(existing: &Claim, incoming: ClaimType) -> bool;

/// Whether a mutation is admitted by the active claim.
pub fn admit_mutation(claim: &Claim, request: &MutationRequest, now: u64, store_ok: bool) -> MutationAdmission;

/// Whether a heartbeat request is valid for the active claim.
pub fn validate_heartbeat(req: &HeartbeatRequest, claim: &Claim, now: u64) -> Result<(), Vec<String>>;

/// Whether a release request is valid.
pub fn validate_release(req: &ReleaseRequest, claim: &Claim) -> Result<(), Vec<String>>;
```

## Part 4 — Claim Store

### Local store (per agent)

File: `.agent-claims.json` (at repo root, gitignored)

```jsonc
{
  "schema": "cynic.agent_claims.v1",
  "agent_id": "hermes:constitution",
  "claims": [
    {
      "claim_id": "claim_001",
      "scope": { "kind": "Crate", "crate_name": "cynic-protocol" },
      "lease_id": "lease_001",
      "fencing_token": 42,
      "granted_at_unix_ms": 1724000000000,
      "expires_at_unix_ms": 1724003600000,
      "status": "active"
    }
  ],
  "last_sync_at_unix_ms": 1724001800000
}
```

**Rule:** The local store is a cache. The kernel store is authoritative.

### Kernel store (SurrealDB)

Table: `agent_claims`

```sql
CREATE agent_claims SET
  claim_id = <unique>,
  agent_id = <string>,
  scope = <object>,
  lease_id = <string>,
  fencing_token = <int>,
  granted_at_unix_ms = <int>,
  expires_at_unix_ms = <int>,
  status = <string>;
```

**Persistence rule:** All mutations to the claim store go through the kernel REST API.
Direct SurrealDB writes bypass coordination and are forbidden for runtime agents.

### Sync protocol

```rust
/// Sync local store with kernel store.
/// Returns the authoritative set of active claims.
pub async fn sync_claims_with_kernel(
    local: &mut LocalClaimStore,
    kernel: &dyn KernelCoordinationPort,
) -> Result<Vec<Claim>, CoordinationError>;
```

## Part 5 — Heartbeat Mechanism

### Background task

Every agent session must spawn a background heartbeat task:

```rust
pub async fn heartbeat_loop(
    agent_id: String,
    claim_id: String,
    lease_id: String,
    fencing_token: u64,
    interval: Duration,
    shutdown: CancellationToken,
) -> Result<(), HeartbeatError> {
    let mut interval = tokio::time::interval(interval);
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = interval.tick() => {
                let proof = heartbeat_claim(&agent_id, &lease_id, fencing_token).await?;
                tracing::info!(lease_id = %lease_id, new_expiry = proof.new_expiry_ms, "heartbeat");
            }
        }
    }
    Ok(())
}
```

### Heartbeat CLI integration

```bash
# Start heartbeat in background (automatic with session start)
cynic agent heartbeat-start --agent-id "hermes:constitution" --interval 300 --json

# Check heartbeat status
cynic agent heartbeat-status --agent-id "hermes:constitution" --json

# Stop heartbeat (graceful)
cynic agent heartbeat-stop --agent-id "hermes:constitution" --json
```

### Heartbeat MCP integration

```json
{
  "tool": "cynic_coord_heartbeat",
  "arguments": {
    "agent_id": "hermes:constitution",
    "lease_id": "lease_001",
    "fencing_token": 42
  }
}
```

## Part 6 — Conflict Resolution

### Automatic conflict detection

```rust
/// Detect conflicts between local claims and kernel state.
pub fn detect_conflicts(local: &[Claim], kernel: &[Claim]) -> Vec<Conflict> {
    let mut conflicts = Vec::new();
    for local_claim in local {
        for kernel_claim in kernel {
            if local_claim.scope.contains(&kernel_claim.scope)
                && local_claim.agent_id != kernel_claim.agent_id
                && kernel_claim.status == ClaimStatus::Active
            {
                conflicts.push(Conflict::new(local_claim, kernel_claim));
            }
        }
    }
    conflicts
}
```

### Conflict resolution strategies

| Conflict | Strategy | Outcome |
|---|---|---|
| Same scope, same mode | First-wins | Second agent picks another scope |
| Cynic vs baseline | Cynic preempts | Baseline gets 300s grace period |
| Overlapping scopes | Most specific wins | Narrow claim proceeds, broad waits |
| Parent-child mismatch | Parent budget check | Child refuses if parent exhausted |
| Budget amplification | Refused | Agent must request new delegation |

### Conflict proof

Every conflict resolution emits a `ConflictProof`:

```rust
pub struct ConflictProof {
    pub schema: String,                    // "cynic.conflict_proof.v1"
    pub conflict_id: String,
    pub local_claim_id: String,
    pub remote_claim_id: String,
    pub resolution: ConflictResolution,    // FirstWins | Preempted | GracePeriod
    pub resolved_at_unix_ms: u64,
    pub evidence_refs: Vec<String>,
}
```

## Part 7 — MCP Tool Inventory

### Complete tool set

| Tool name | Phase | Description | Mutates |
|---|---|---|---|
| `cynic_agent_identify` | IDENTIFY | Declare identity | No |
| `cynic_coord_register` | REGISTER | Register with kernel | Yes |
| `cynic_coord_claim` | CLAIM | Claim a scope | Yes |
| `cynic_coord_heartbeat` | HEARTBEAT | Renew claim TTL | Yes |
| `cynic_coord_release` | RELEASE | Release a claim | Yes |
| `cynic_coord_who` | STATUS | Query active claims | No |
| `cynic_coord_verify` | PROOF | Verify a claim | No |

### Tool schemas (JSON)

#### cynic_agent_identify

```json
{
  "inputSchema": {
    "type": "object",
    "properties": {
      "agent_id": { "type": "string" },
      "agent_kind": { "type": "string", "enum": ["hermes", "codex", "claude", "cynic_native", "human_admin"] },
      "credential_kind": { "type": "string", "enum": ["local_peer_binding", "opaque_reference", "signed_jwt"] },
      "credential_reference": { "type": "string" }
    },
    "required": ["agent_id"]
  }
}
```

#### cynic_coord_claim

```json
{
  "inputSchema": {
    "type": "object",
    "properties": {
      "agent_id": { "type": "string" },
      "scope": { "type": "object", "oneOf": [...] },
      "claim_type": { "type": "string", "enum": ["exclusive", "shared_observe"] },
      "ttl_seconds": { "type": "integer", "minimum": 30, "maximum": 86400 },
      "budget_actions": { "type": "integer", "minimum": 1 },
      "budget_wall_seconds": { "type": "integer", "minimum": 60 },
      "max_lease_window_ms": { "type": "integer", "minimum": 60000 },
      "parent_claim_id": { "type": "string" }
    },
    "required": ["agent_id", "scope", "ttl_seconds"]
  }
}
```

#### cynic_coord_heartbeat

```json
{
  "inputSchema": {
    "type": "object",
    "properties": {
      "agent_id": { "type": "string" },
      "lease_id": { "type": "string" },
      "fencing_token": { "type": "integer", "minimum": 0 }
    },
    "required": ["agent_id", "lease_id", "fencing_token"]
  }
}
```

#### cynic_coord_release

```json
{
  "inputSchema": {
    "type": "object",
    "properties": {
      "agent_id": { "type": "string" },
      "lease_id": { "type": "string" },
      "fencing_token": { "type": "integer", "minimum": 0 },
      "proof_refs": { "type": "array", "items": { "type": "string" } }
    },
    "required": ["agent_id", "lease_id", "fencing_token"]
  }
}
```

## Part 8 — CLI Command Inventory

### Complete command set

```bash
# Lifecycle
cynic agent identify --agent-id <id> [--agent-kind <kind>] [--json]
cynic agent register --agent-id <id> --intent <text> [--json]
cynic agent claim --agent-id <id> --target <scope> [--claim-type <type>] [--ttl-seconds <n>] [--json]
cynic agent heartbeat --agent-id <id> --lease-id <id> --fencing-token <n> [--json]
cynic agent release --agent-id <id> --lease-id <id> --fencing-token <n> [--proof-ref <ref> ...] [--json]

# Query
cynic agent status --agent-id <id> [--json]
cynic agent who [--agent-id <id>] [--json]

# Background
cynic agent heartbeat-start --agent-id <id> [--interval <secs>] [--json]
cynic agent heartbeat-stop --agent-id <id> [--json]

# Git (from agent-git-protocol)
cynic agent branch-create --scope <scope> [--json]
cynic agent branch-sync --json
cynic agent branch-push --json
cynic agent pr-create --title <title> [--body <body>] [--json]
```

### CLI output schema

All commands emit JSON with a `schema` field:

```json
{
  "schema": "cynic.cli.agent_claim_report.v1",
  "status": "claimed",
  "agent_id": "hermes:constitution",
  "target": "crates/cynic-protocol",
  "claim_type": "file",
  "lease_id": "lease_001",
  "fencing_token": 42,
  "ttl_seconds": 1800,
  "budget_actions": 50,
  "budget_wall_seconds": 3600,
  "granted_at_unix_ms": 1724000000000,
  "expires_at_unix_ms": 1724001800000
}
```

## Part 9 — Kernel REST Endpoints

### Complete endpoint map

| Method | Path | Handler | Auth |
|---|---|---|---|
| POST | `/coord/identify` | `coord_identify_handler` | No |
| POST | `/coord/register` | `coord_register_handler` | Yes |
| POST | `/coord/claim` | `coord_claim_handler` | Yes |
| POST | `/coord/renew` | `coord_renew_handler` | Yes |
| POST | `/coord/release` | `coord_release_handler` | Yes |
| POST | `/coord/heartbeat` | `coord_heartbeat_handler` | Yes |
| GET | `/coord/who` | `coord_who_handler` | No |
| GET | `/coord/verify` | `coord_verify_handler` | Yes |

### Request/Response types

```rust
// Identify
pub struct AgentIdentityRequest {
    pub agent_id: String,
    pub agent_kind: Option<String>,
    pub credential_kind: Option<String>,
    pub credential_reference: Option<String>,
}

// Claim
pub struct ClaimRequest {
    pub agent_id: String,
    pub session_id: String,
    pub scope: ClaimScope,
    pub claim_type: Option<String>,
    pub ttl_seconds: Option<u64>,
    pub budget_actions: Option<u64>,
    pub budget_wall_seconds: Option<u64>,
    pub max_lease_window_ms: Option<u64>,
    pub parent_claim_id: Option<String>,
}

// Heartbeat
pub struct HeartbeatRequest {
    pub agent_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
}

// Release
pub struct ReleaseRequest {
    pub agent_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub proof_refs: Vec<String>,
}
```

## Part 10 — Proofs

### Proof taxonomy

| Proof type | When produced | Consumed by |
|---|---|---|
| `ClaimProof` | After successful claim | Higher layers (audit) |
| `HeartbeatProof` | Each heartbeat tick | Monitoring/alerting |
| `ReleaseProof` | After successful release | Merge validation |
| `MutationProof` | After each bounded mutation | Read-back verification |
| `ConflictProof` | After conflict resolution | Audit trail |
| `SyncProof` | After local-kernel sync | Consistency check |

### Proof schema

```rust
pub struct ProofEnvelope {
    pub schema: String,                    // "cynic.proof.envelope.v1"
    pub proof_id: String,
    pub kind: ProofKind,
    pub subject_id: String,
    pub produced_at_unix_ms: u64,
    pub producer_id: String,
    pub evidence_refs: Vec<String>,
    pub digest: String,                    // sha256 of canonical payload
}
```

## Part 11 — Local Store Synchronization

### Sync algorithm

```
1. READ local .agent-claims.json
2. READ kernel GET /coord/who?agent_id=<id>
3. DIFF local vs kernel
4. FOR EACH local claim not in kernel → mark EXPIRED
5. FOR EACH kernel claim not in local → ADD to local
6. FOR EACH matching claim → UPDATE local with kernel state
7. WRITE local .agent-claims.json
```

### Sync trigger

- Before any mutation
- After any session start
- Periodically (every 5 min via background task)
- On kernel reconnection after disconnect

## Part 12 — Error Handling

### Error hierarchy

```rust
pub enum AgentProtocolError {
    IdentityInvalid(Vec<String>),
    RegistrationFailed,
    ClaimRefused(Vec<ClaimConflict>),
    ClaimExpired,
    ClaimPreempted { by: String },
    HeartbeatFailed,
    ReleaseFailed,
    MutationNotAdmitted(Vec<CoordinationCode>),
    StoreUnavailable,
    KernelUnreachable,
    BudgetAmplification,
    ScopeConflict,
}
```

### Error recovery

| Error | Recovery | Max retries |
|---|---|---|
| `ClaimExpired` | Re-claim | 3 |
| `ClaimPreempted` | Wait grace period, then re-claim | 1 |
| `HeartbeatFailed` | Retry with backoff | 5 |
| `StoreUnavailable` | Switch to local-only mode | N/A |
| `KernelUnreachable` | Cache locally, retry sync | 10 |

## Part 13 — Implementation Checklist

### Protocol types (cynic-protocol)

- [x] `AgentIdentity`, `AgentKind`, `CredentialAttestation`
- [x] `ClaimRequest`, `Claim`, `ClaimScope`, `ClaimType`, `ClaimStatus`
- [x] `HeartbeatRequest`, `HeartbeatProof`
- [x] `ReleaseRequest`, `ReleaseProof`
- [x] `MutationRequest`, `MutationAction`, `MutationAdmission`
- [x] `CoordinationCode`
- [ ] `Conflict`, `ConflictResolution`, `ConflictProof`
- [ ] `ClaimProof`, `ReleaseProof`, `MutationProof`
- [ ] `ProofEnvelope`, `ProofKind`
- [ ] `LocalClaimStore`, `ClaimSyncRequest`
- [ ] Pure predicates: `detect_conflicts`, `resolve_conflict`

### Kernel handlers (cynic-kernel)

- [x] `coord_identify_handler`
- [x] `coord_register_handler`
- [x] `coord_claim_handler`
- [x] `coord_renew_handler` (heartbeat)
- [x] `coord_release_handler`
- [x] `coord_heartbeat_handler`
- [x] `coord_who_handler`
- [ ] `coord_verify_handler` (enhanced with proof)
- [ ] Durable claim store in SurrealDB

### MCP tools (cynic-mcp)

- [x] `cynic_agent_identify`
- [x] `cynic_coord_register`
- [x] `cynic_coord_claim`
- [x] `cynic_coord_renew`
- [x] `cynic_coord_release`
- [x] `cynic_coord_who`
- [ ] `cynic_coord_heartbeat` (dedicated tool)
- [ ] `cynic_coord_verify` (dedicated tool)

### CLI commands (cynic-cli)

- [x] `cynic agent identify`
- [x] `cynic agent register`
- [x] `cynic agent claim`
- [x] `cynic agent release`
- [x] `cynic agent status`
- [ ] `cynic agent heartbeat`
- [ ] `cynic agent who`
- [ ] `cynic agent heartbeat-start/stop/status`

### Documentation

- [x] `protocols/agent-constitution.md`
- [x] `protocols/agent-git-protocol.md`  ← now exists; previously a missing citation (see it for commit/branch/PR rules)
- [x] `protocols/agent-coordination-protocol.md`
- [ ] `protocols/agent-protocol.md` (this document)

---

*This specification is executable prose. Law lives in Rust types + tests + CLI surfaces.
Transpose, compile, test, merge.*
