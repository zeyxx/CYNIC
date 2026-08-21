# CYNIC Git Multi-Source Plane — Layer 1: Data Model

## Purpose

This document formalizes the **Git multi-source data model** for CYNIC.
It defines how multiple Git remotes (SSOT + relays) are represented,
synchronized, and coordinated across agents without introducing a SPOF.

**Scope:** Types, schemas, state machine, integration points with existing surfaces.

**Out of scope:** Implementation details (adapters, CLI, MCP tools) — those are Layers 2-5.

## 1. Existing Integration Map

Before defining new types, we map what already exists:

| Component | Current role | Integration point |
|---|---|---|
| `infra/repo-remotes.json` | Declarative remote registry (Forgejo SSOT + GitHub relay) | Source of truth for remote URLs, roles, credentials |
| `cynic-mcp::cynic_git` | Read-only git observations (status, diff, log) | Extensible for push/sync/PR |
| `cynic-mcp::cynic_source_control` | Source mutation (commit, sync) with lease admission | Extensible for multi-remote push |
| `cynic-cli` | CLI surfaces (58 git-related commands) | Extensible for `cynic git sync/push/pr` |
| `cynic-protocol` | No Git types exist | **This layer adds them** |
| `cynic-host` | No Git adapters exist | Layer 2 adds `GitRemotePort` |
| `cynic-kernel` | No Git store exists | Layer 3 adds optional SurrealDB store |

## 2. Core Types

### 2.1 GitRemote

Represents a single Git remote endpoint.

```rust
pub struct GitRemote {
    /// Unique identifier (e.g., "forgejo", "origin", "custom-mirror").
    pub id: String,

    /// Role in the multi-source topology.
    pub role: GitRemoteRole,

    /// Git URL (SSH or HTTPS).
    pub url: String,

    /// Credential reference (file path or env var name).
    /// Never the credential itself.
    pub credential_ref: String,

    /// Current observed status.
    pub status: GitRemoteStatus,

    /// Last successful sync timestamp (unix ms).
    pub last_sync_at_unix_ms: Option<u64>,

    /// Priority for sync order (lower = synced first).
    pub priority: u8,
}

pub enum GitRemoteRole {
    /// Sovereign SSOT — mutations (push, PR) are authoritative.
    /// There is exactly one active SovereignCandidate at a time.
    SovereignCandidate,

    /// Public relay — read-only mirror or push-only mirror.
    /// Never the target of PR creation.
    PublicRelay,

    /// Archive — read-only, no push.
    Archive,
}

pub enum GitRemoteStatus {
    /// Reachable and authenticated.
    Reachable,

    /// Reachable but authentication failed.
    Unauthorized,

    /// Not reachable (network down, DNS failure).
    Unreachable,

    /// Reachable but role mismatch (e.g., SSOT declared but read-only).
    Degraded,
}
```

### 2.2 GitSyncState

Represents the synchronization state between local and remotes.

```rust
pub struct GitSyncState {
    /// Local HEAD commit SHA.
    pub local_head: String,

    /// Local branch name.
    pub local_branch: String,

    /// Per-remote sync status.
    pub remote_states: Vec<RemoteSyncState>,

    /// Whether local has diverged from any remote.
    pub diverged: bool,

    /// Whether local is behind any remote (needs pull).
    pub behind: bool,

    /// Whether local is ahead of any remote (needs push).
    pub ahead: bool,
}

pub struct RemoteSyncState {
    pub remote_id: String,
    pub remote_head: String,
    pub behind_count: u64,
    pub ahead_count: u64,
    pub diverged: bool,
    pub last_sync_at_unix_ms: Option<u64>,
}
```

### 2.3 MultiAgentGitClaim

Extends the existing `Claim` type with Git-specific metadata.

```rust
pub struct MultiAgentGitClaim {
    /// Standard claim fields (from agent_coordination::Claim).
    pub claim: Claim,

    /// The remote this claim applies to.
    pub remote_id: String,

    /// The branch being claimed.
    pub branch: String,

    /// Whether this claim authorizes push.
    pub push_authorized: bool,

    /// Whether this claim authorizes PR creation.
    pub pr_authorized: bool,
}
```

### 2.4 GitOperation

Represents a bounded Git operation (push, sync, PR).

```rust
pub struct GitOperation {
    pub schema: String,                    // "cynic.git_operation.v1"
    pub operation_id: String,
    pub agent_id: String,
    pub claim_id: String,
    pub remote_id: String,
    pub branch: String,
    pub action: GitAction,
    pub status: GitOperationStatus,
    pub evidence_refs: Vec<String>,
    pub initiated_at_unix_ms: u64,
    pub completed_at_unix_ms: Option<u64>,
}

pub enum GitAction {
    Push,
    Fetch,
    Pull,
    Sync,
    PrCreate,
    PrMerge,
}

pub enum GitOperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed { reason: String },
    Cancelled,
}
```

## 3. State Machine

### 3.1 Remote lifecycle

```
UNINITIALIZED → DISCOVERED → REACHABLE → SYNCING → SYNCED
                  ↓              ↓           ↓
               FAILED      DEGRADED     DIVERGED
```

### 3.2 Sync operation lifecycle

```
IDLE → FETCHING → COMPARING → PUSHING → DONE
         ↓           ↓          ↓
      FAILED      DIVERGED   CONFLICT
```

### 3.3 Multi-agent coordination

```
Agent A claims forgejo:branch-a → ACTIVE
Agent B claims forgejo:branch-b → ACTIVE (different branch, no conflict)
Agent B tries forgejo:branch-a  → REFUSED (first-wins)
Agent A releases forgejo:branch-a → RELEASED
Agent B retries forgejo:branch-a → ACTIVE (claim available)
```

## 4. Integration with Existing Surfaces

### 4.1 `infra/repo-remotes.json` → `GitRemote`

The JSON file is the **declarative source of truth**.
At startup, surfaces read this file to discover remotes.

```rust
pub fn load_git_remotes_from_infra() -> Result<Vec<GitRemote>, String> {
    let content = std::fs::read_to_string("infra/repo-remotes.json")...;
    // Parse JSON into Vec<GitRemote>
}
```

**Rule:** The JSON file is read-only at runtime. Mutations go through `cynic admin remotes add/remove` (future surface).

### 4.2 `cynic-mcp::cynic_git` → Extended

The existing `cynic_git` tool is read-only. We extend it with:

- `cynic_git_push` — push to a specific remote (requires claim)
- `cynic_git_sync` — sync all remotes (SSOT first, then relays)
- `cynic_git_pr` — create PR on SSOT

### 4.3 `cynic-mcp::cynic_source_control` → Integrated

The existing `cynic_source_control` handles source mutation with lease admission.
We integrate multi-remote push into this flow:

1. Agent holds claim on scope
2. Agent calls `cynic_source_control` to commit
3. Agent calls `cynic_git_push` to push to SSOT
4. Agent calls `cynic_git_push` to push to relay (if configured)

### 4.4 `cynic-cli` → Extended

New commands:

```bash
cynic git status --json              # Multi-remote status
cynic git sync --json                # Sync all remotes
cynic git push --remote <id> --json  # Push to specific remote
cynic git pr create --json           # Create PR on SSOT
```

## 5. Anti-SPOF Design

### 5.1 No single point of failure

| Component | Failure mode | Degraded behavior |
|---|---|---|
| `infra/repo-remotes.json` | File missing | Use hardcoded defaults (Forgejo + GitHub) |
| Kernel store (SurrealDB) | DB unreachable | Use local `.agent-claims.json` file |
| SSOT remote (Forgejo) | Unreachable | Queue push, retry with backoff |
| Relay remote (GitHub) | Unreachable | Skip, log warning, continue |
| Claim store | Unavailable | Refuse mutation (fail-closed) |

### 5.2 Local-first architecture

```
Local .agent-claims.json  ← authoritative when kernel unreachable
        ↓ sync when available
Kernel SurrealDB          ← authoritative when reachable
        ↓ observe
infra/repo-remotes.json   ← static configuration (read-only)
```

### 5.3 Deterministic sync order

1. **SSOT first** (Forgejo) — push/pull/PR always targets SSOT
2. **Relays second** (GitHub) — push-only, no PR creation
3. **Archives last** — read-only fetch

This ensures that even if a relay fails, the SSOT is consistent.

## 6. Conflict Scenarios

### 6.1 Divergent branches

```
Local:  A---B---C (main)
Forgejo: A---B---D (main)
```

**Resolution:** `cynic git sync` detects divergence, refuses automatic merge, asks agent to resolve manually.

### 6.2 Concurrent push to SSOT

```
Agent A pushes branch-a to forgejo → SUCCESS
Agent B pushes branch-b to forgejo → SUCCESS (different branch)
Agent B pushes branch-a to forgejo → REFUSED (Agent A holds claim)
```

**Resolution:** Claims are per-scope. Different branches = different scopes.

### 6.3 SSOT unreachable

```
Agent A tries to push to forgejo → TIMEOUT
```

**Resolution:** Push is queued locally. Background task retries with exponential backoff. Agent continues working (optimistic local-first).

### 6.4 Relay push fails

```
Agent A pushes to forgejo → SUCCESS
Agent A pushes to github → AUTH_FAILURE
```

**Resolution:** Log warning, continue. Relay is optional. SSOT is consistent.

## 7. Schema Versions

| Schema | Version | Status |
|---|---|---|
| `cynic.git_remote.v1` | 1.0.0 | Proposed |
| `cynic.git_sync_state.v1` | 1.0.0 | Proposed |
| `cynic.git_operation.v1` | 1.0.0 | Proposed |
| `cynic.git_claim.v1` | 1.0.0 | Proposed |

## 8. Open Questions

1. **Should GitRemote be stored in SurrealDB or only in JSON?**
   → Proposal: JSON is source of truth, SurrealDB is optional cache.

2. **How to handle credential rotation?**
   → Proposal: Credential refs are stable (file paths). Rotation updates file content, not ref.

3. **Should we support more than one SSOT?**
   → Proposal: No. One SSOT at a time. Migration = change JSON + re-sync.

4. **How to handle large repos (git LFS)?**
   → Proposal: Out of scope for Layer 1. Document as known limitation.

---

*This document is Layer 1 of 5. It defines the data model only.
Layer 2 (adapters), Layer 3 (kernel store), Layer 4 (MCP tools), Layer 5 (CLI)
will reference this document.*
