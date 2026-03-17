# Kernel Refactor Phase 1 — Fix Architectural Violations

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 3 architectural violations (V1, V4, V2+V3) so the codebase is clean before Phase 2 module reorganization.

**Architecture:** Extract shared types to standalone files, create missing port traits, replace concrete adapter references with trait objects. Each task produces a compilable, tested commit. Zero behavioral changes.

**Tech Stack:** Rust, async_trait, serde, serde_json, axum, rmcp, reqwest, tokio

**Spec:** `docs/superpowers/specs/2026-03-16-kernel-modular-refactor-design.md`

**Build command:** `source ~/.cargo/env && cargo build -p cynic-kernel --release && cargo test -p cynic-kernel --release && cargo clippy -p cynic-kernel --release -- -D warnings`

---

## Chunk 1: Fix V1 + Fix V4

### Task 1: Extract DogUsageTracker from rest.rs (Fix V1)

**Problem:** `mcp.rs:28` imports `use crate::rest::DogUsageTracker` — cross-adapter coupling.

**Files:**
- Create: `cynic-kernel/src/usage.rs`
- Modify: `cynic-kernel/src/rest.rs` (remove DogUsageTracker + DogUsage definitions)
- Modify: `cynic-kernel/src/mcp.rs` (change import)
- Modify: `cynic-kernel/src/main.rs` (change import)
- Modify: `cynic-kernel/src/lib.rs` (add module declaration)

- [ ] **Step 1: Create `cynic-kernel/src/usage.rs` with DogUsageTracker + DogUsage**

```rust
//! DogUsageTracker — tracks token consumption and request counts per Dog.
//! Shared by REST and MCP adapters. Domain-level concern (Dog performance metrics).

use std::collections::HashMap;

/// Tracks token consumption and request counts per Dog since boot.
pub struct DogUsageTracker {
    pub dogs: HashMap<String, DogUsage>,
    pub boot_time: chrono::DateTime<chrono::Utc>,
    pub total_requests: u64,
}

#[derive(Default, Clone)]
pub struct DogUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub requests: u64,
    pub failures: u64,
    pub total_latency_ms: u64,
}

impl Default for DogUsageTracker {
    fn default() -> Self { Self::new() }
}

impl DogUsageTracker {
    pub fn new() -> Self {
        Self {
            dogs: HashMap::new(),
            boot_time: chrono::Utc::now(),
            total_requests: 0,
        }
    }

    pub fn record(&mut self, dog_id: &str, prompt: u32, completion: u32, latency_ms: u64) {
        let entry = self.dogs.entry(dog_id.to_string()).or_default();
        entry.prompt_tokens += prompt as u64;
        entry.completion_tokens += completion as u64;
        entry.requests += 1;
        entry.total_latency_ms += latency_ms;
    }

    pub fn record_failure(&mut self, dog_id: &str) {
        let entry = self.dogs.entry(dog_id.to_string()).or_default();
        entry.failures += 1;
    }

    pub fn total_tokens(&self) -> u64 {
        self.dogs.values().map(|d| d.prompt_tokens + d.completion_tokens).sum()
    }

    /// Estimated cost in USD (rough average: $0.15/1M tokens)
    pub fn estimated_cost_usd(&self) -> f64 {
        self.total_tokens() as f64 * 0.15 / 1_000_000.0
    }

    pub fn uptime_seconds(&self) -> i64 {
        (chrono::Utc::now() - self.boot_time).num_seconds()
    }
}
```

- [ ] **Step 2: Add module declaration to `lib.rs`**

Add after the existing module declarations (after line 39 `pub mod mcp;`):
```rust
pub mod usage;
```

- [ ] **Step 3: Remove DogUsageTracker + DogUsage from `rest.rs`**

Delete lines 38-92 from `cynic-kernel/src/rest.rs` (everything from `/// Tracks token consumption` through the closing brace of `impl DogUsageTracker`).

Add at the top of `rest.rs` (in the imports section):
```rust
use crate::usage::{DogUsageTracker, DogUsage};
```

- [ ] **Step 4: Update import in `mcp.rs`**

Change line 28 from:
```rust
use crate::rest::DogUsageTracker;
```
to:
```rust
use crate::usage::DogUsageTracker;
```

- [ ] **Step 5: Update import in `main.rs`**

Change line 147 from:
```rust
let usage_tracker = Arc::new(std::sync::Mutex::new(rest::DogUsageTracker::new()));
```
to:
```rust
let usage_tracker = Arc::new(std::sync::Mutex::new(usage::DogUsageTracker::new()));
```

- [ ] **Step 6: Build + test + clippy**

Run: `source ~/.cargo/env && cargo build -p cynic-kernel --release && cargo test -p cynic-kernel --release && cargo clippy -p cynic-kernel --release -- -D warnings`

Expected: All pass. Zero behavioral change.

- [ ] **Step 7: Commit**

```bash
git add cynic-kernel/src/usage.rs cynic-kernel/src/lib.rs cynic-kernel/src/rest.rs cynic-kernel/src/mcp.rs cynic-kernel/src/main.rs
git commit -m "refactor(domain): extract DogUsageTracker from rest adapter — fix V1 cross-adapter coupling"
```

---

### Task 2: Create InferenceRouter trait (Fix V4)

**Problem:** `hal.rs:9` uses `Arc<BackendRouter>` concretely instead of a trait. Couples gRPC adapter to a specific router implementation.

**Files:**
- Modify: `cynic-kernel/src/backend.rs` (add InferenceRouter trait)
- Modify: `cynic-kernel/src/router.rs` (implement InferenceRouter for BackendRouter)
- Modify: `cynic-kernel/src/hal.rs` (use `Arc<dyn InferenceRouter>` instead of `Arc<BackendRouter>`)
- Modify: `cynic-kernel/src/main.rs` (cast BackendRouter to dyn InferenceRouter)

- [ ] **Step 1: Add InferenceRouter trait to `backend.rs`**

Add after the `InferencePort` trait definition (after line 122, before `use async_trait::async_trait;`). Since `async_trait` is already imported at line 124, add the trait just before the MockBackend section:

```rust
/// Router contract — selects and dispatches to registered InferencePort backends.
/// Domain-level trait: the gRPC layer sees this, not the concrete BackendRouter.
#[async_trait]
pub trait InferenceRouter: Send + Sync {
    async fn route(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError>;
    async fn fan_out(&self, req: InferenceRequest, n: u32) -> Result<InferenceResponse, BackendError>;
}
```

- [ ] **Step 2: Implement InferenceRouter for BackendRouter in `router.rs`**

Add at the end of the file, before the `#[cfg(test)]` module:

```rust
#[async_trait::async_trait]
impl crate::backend::InferenceRouter for BackendRouter {
    async fn route(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError> {
        self.route(req).await
    }
    async fn fan_out(&self, req: InferenceRequest, n: u32) -> Result<InferenceResponse, BackendError> {
        self.fan_out(req, n).await
    }
}
```

**Note:** The trait methods have the same name as the inherent methods. Rust resolves inherent methods first, so `self.route(req)` inside the trait impl calls the inherent `BackendRouter::route`. This is the correct delegation pattern.

- [ ] **Step 3: Update `hal.rs` to use the trait**

Change line 9 from:
```rust
use crate::router::BackendRouter;
```
to:
```rust
use crate::backend::InferenceRouter;
```

Change line 16 from:
```rust
    router: Arc<BackendRouter>,
```
to:
```rust
    router: Arc<dyn InferenceRouter>,
```

Change line 20-22 from:
```rust
    pub fn new(router: Arc<BackendRouter>) -> Self {
```
to:
```rust
    pub fn new(router: Arc<dyn InferenceRouter>) -> Self {
```

- [ ] **Step 4: Update `main.rs` — cast BackendRouter to dyn InferenceRouter**

Line 102 currently creates:
```rust
let router = Arc::new(router::BackendRouter::new(vec![]));
```

Change line 190 from:
```rust
    let muscle_service = hal::MuscleService::new(Arc::clone(&router));
```
to:
```rust
    let muscle_service = hal::MuscleService::new(Arc::clone(&router) as Arc<dyn backend::InferenceRouter>);
```

- [ ] **Step 5: Build + test + clippy**

Run: `source ~/.cargo/env && cargo build -p cynic-kernel --release && cargo test -p cynic-kernel --release && cargo clippy -p cynic-kernel --release -- -D warnings`

Expected: All pass. Zero behavioral change.

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/backend.rs cynic-kernel/src/router.rs cynic-kernel/src/hal.rs cynic-kernel/src/main.rs
git commit -m "refactor(domain): InferenceRouter trait — decouple hal from concrete BackendRouter (fix V4)"
```

---

## Chunk 2: Fix V2+V3 — CoordPort

### Task 3: Create CoordPort trait + NullCoord (domain layer)

**Problem:** Both `rest.rs` and `mcp.rs` hold `raw_db: Option<Arc<SurrealHttpStorage>>` to run raw SurrealQL for coordination (agent sessions, work claims) and audit (mcp_audit). This bypasses the port abstraction.

**Solution:** Create `CoordPort` trait in a new file. `SurrealHttpStorage` implements it. REST and MCP receive `Arc<dyn CoordPort>` instead of `raw_db`.

**Files:**
- Create: `cynic-kernel/src/coord_port.rs`
- Modify: `cynic-kernel/src/lib.rs` (add module declaration)

- [ ] **Step 1: Create `cynic-kernel/src/coord_port.rs`**

```rust
//! CoordPort — domain contract for agent coordination and audit.
//! Covers agent sessions, work claims, and MCP audit trail.
//! SurrealHttpStorage implements this. REST and MCP use Arc<dyn CoordPort>.

use async_trait::async_trait;

#[derive(Debug)]
pub enum CoordError {
    StorageFailed(String),
    InvalidInput(String),
}

impl std::fmt::Display for CoordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StorageFailed(m) => write!(f, "Coordination storage failed: {}", m),
            Self::InvalidInput(m) => write!(f, "Invalid input: {}", m),
        }
    }
}

/// Result of a claim attempt.
pub enum ClaimResult {
    /// Claim granted.
    Claimed,
    /// Another agent holds this target.
    Conflict(Vec<ConflictInfo>),
}

pub struct ConflictInfo {
    pub agent_id: String,
    pub claimed_at: String,
}

/// Snapshot of active agents and claims.
pub struct CoordSnapshot {
    pub agents: Vec<serde_json::Value>,
    pub claims: Vec<serde_json::Value>,
}

#[async_trait]
pub trait CoordPort: Send + Sync {
    /// Register an agent session. Upserts — safe to call multiple times.
    async fn register_agent(&self, agent_id: &str, agent_type: &str, intent: &str) -> Result<(), CoordError>;

    /// Claim a file/feature/zone. Returns Conflict if another agent holds it.
    async fn claim(&self, agent_id: &str, target: &str, claim_type: &str) -> Result<ClaimResult, CoordError>;

    /// Release claims. If target is None, releases ALL claims for this agent.
    async fn release(&self, agent_id: &str, target: Option<&str>) -> Result<String, CoordError>;

    /// Show active agents and claims. Expires stale sessions first (>5 min).
    async fn who(&self, agent_id_filter: Option<&str>) -> Result<CoordSnapshot, CoordError>;

    /// Store an audit entry (best-effort, non-critical).
    async fn store_audit(&self, tool: &str, agent_id: &str, details: &serde_json::Value) -> Result<(), CoordError>;

    /// Query audit trail with optional filters.
    async fn query_audit(&self, tool_filter: Option<&str>, agent_filter: Option<&str>, limit: u32) -> Result<Vec<serde_json::Value>, CoordError>;

    /// Refresh agent heartbeat (called on every MCP tool invocation).
    async fn heartbeat(&self, agent_id: &str) -> Result<(), CoordError>;

    /// Mark agent session inactive + release all claims.
    async fn deactivate_agent(&self, agent_id: &str) -> Result<(), CoordError>;
}

/// No-op coordination for graceful degradation when DB is unavailable.
pub struct NullCoord;

#[async_trait]
impl CoordPort for NullCoord {
    async fn register_agent(&self, _: &str, _: &str, _: &str) -> Result<(), CoordError> { Ok(()) }
    async fn claim(&self, _: &str, _: &str, _: &str) -> Result<ClaimResult, CoordError> {
        Ok(ClaimResult::Claimed) // optimistic in degraded mode
    }
    async fn release(&self, _: &str, _: Option<&str>) -> Result<String, CoordError> {
        Ok("Released (degraded mode)".into())
    }
    async fn who(&self, _: Option<&str>) -> Result<CoordSnapshot, CoordError> {
        Ok(CoordSnapshot { agents: vec![], claims: vec![] })
    }
    async fn store_audit(&self, _: &str, _: &str, _: &serde_json::Value) -> Result<(), CoordError> { Ok(()) }
    async fn query_audit(&self, _: Option<&str>, _: Option<&str>, _: u32) -> Result<Vec<serde_json::Value>, CoordError> {
        Ok(vec![])
    }
    async fn heartbeat(&self, _: &str) -> Result<(), CoordError> { Ok(()) }
    async fn deactivate_agent(&self, _: &str) -> Result<(), CoordError> { Ok(()) }
}
```

- [ ] **Step 2: Add module declaration to `lib.rs`**

Add after the `pub mod usage;` line:
```rust
pub mod coord_port;
```

- [ ] **Step 3: Build to verify trait compiles**

Run: `source ~/.cargo/env && cargo build -p cynic-kernel --release`

Expected: Compiles (no consumers yet).

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/coord_port.rs cynic-kernel/src/lib.rs
git commit -m "refactor(domain): add CoordPort trait + NullCoord for agent coordination"
```

---

### Task 4: Implement CoordPort for SurrealHttpStorage

**Files:**
- Modify: `cynic-kernel/src/storage_http.rs` (add `impl CoordPort`)

This moves all raw SurrealQL for coordination and audit from `mcp.rs` and `rest.rs` into the storage adapter where it belongs.

- [ ] **Step 1: Add CoordPort import to `storage_http.rs`**

Add to the imports at the top of the file:
```rust
use crate::coord_port::{CoordPort, CoordError, ClaimResult, ConflictInfo, CoordSnapshot};
```

- [ ] **Step 2: Implement CoordPort for SurrealHttpStorage**

Add at the end of the file, before `#[cfg(test)]`:

```rust
// ── COORD PORT IMPLEMENTATION ──────────────────────────────

/// Sanitize an ID for use in SurrealDB backtick record identifiers.
/// Replaces non-alphanumeric chars (except _) with _ to prevent parse errors.
fn sanitize_record_id(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect()
}

#[async_trait::async_trait]
impl CoordPort for SurrealHttpStorage {
    async fn register_agent(&self, agent_id: &str, agent_type: &str, intent: &str) -> Result<(), CoordError> {
        let escape = |s: &str| escape_surreal(s);
        let sql = format!(
            "UPSERT agent_session:`{id_key}` SET \
                agent_id = '{id_val}', \
                agent_type = '{agent_type}', \
                intent = '{intent}', \
                registered_at = time::now(), \
                last_seen = time::now(), \
                active = true;",
            id_key = sanitize_record_id(agent_id),
            id_val = escape(agent_id),
            agent_type = escape(agent_type),
            intent = escape(intent),
        );
        self.query_one(&sql).await
            .map_err(|e| CoordError::StorageFailed(format!("Register failed: {}", e)))?;
        Ok(())
    }

    async fn claim(&self, agent_id: &str, target: &str, claim_type: &str) -> Result<ClaimResult, CoordError> {
        let escape = |s: &str| escape_surreal(s);

        // Check for existing active claims by OTHER agents
        let check_sql = format!(
            "SELECT * FROM work_claim WHERE target = '{}' AND agent_id != '{}' AND active = true;",
            escape(target), escape(agent_id)
        );
        let conflicts = self.query_one(&check_sql).await
            .map_err(|e| CoordError::StorageFailed(format!("Claim check failed: {}", e)))?;

        if !conflicts.is_empty() {
            let infos = conflicts.iter().map(|c| ConflictInfo {
                agent_id: c["agent_id"].as_str().unwrap_or("?").to_string(),
                claimed_at: c["claimed_at"].as_str().unwrap_or("?").to_string(),
            }).collect();
            return Ok(ClaimResult::Conflict(infos));
        }

        // Create or update the claim
        let claim_id = format!("{}_{}", agent_id, target.replace(['/', '.'], "_"));
        let sql = format!(
            "UPSERT work_claim:`{claim_id_key}` SET \
                agent_id = '{agent_id}', \
                target = '{target}', \
                claim_type = '{claim_type}', \
                claimed_at = time::now(), \
                active = true;",
            claim_id_key = sanitize_record_id(&claim_id),
            agent_id = escape(agent_id),
            target = escape(target),
            claim_type = escape(claim_type),
        );
        self.query_one(&sql).await
            .map_err(|e| CoordError::StorageFailed(format!("Claim failed: {}", e)))?;

        // Refresh heartbeat
        let _ = self.heartbeat(agent_id).await;
        Ok(ClaimResult::Claimed)
    }

    async fn release(&self, agent_id: &str, target: Option<&str>) -> Result<String, CoordError> {
        let escape = |s: &str| escape_surreal(s);

        let (sql, desc) = match target {
            Some(t) => (
                format!("UPDATE work_claim SET active = false WHERE agent_id = '{}' AND target = '{}' AND active = true;",
                    escape(agent_id), escape(t)),
                format!("Released '{}' for agent '{}'.", t, agent_id),
            ),
            None => (
                format!("UPDATE work_claim SET active = false WHERE agent_id = '{}' AND active = true;",
                    escape(agent_id)),
                format!("Released ALL claims for agent '{}'.", agent_id),
            ),
        };

        self.query_one(&sql).await
            .map_err(|e| CoordError::StorageFailed(format!("Release failed: {}", e)))?;

        // If releasing all, also deactivate the session
        if target.is_none() {
            let _ = self.deactivate_agent(agent_id).await;
        }

        Ok(desc)
    }

    async fn who(&self, agent_id_filter: Option<&str>) -> Result<CoordSnapshot, CoordError> {
        let escape = |s: &str| escape_surreal(s);

        // Expire stale sessions (>5 min since last_seen)
        let _ = self.query_one(
            "UPDATE agent_session SET active = false WHERE active = true AND (time::now() - last_seen) > 5m;"
        ).await;
        // Expire claims from inactive agents
        let _ = self.query_one(
            "UPDATE work_claim SET active = false WHERE active = true AND agent_id NOT IN (SELECT VALUE agent_id FROM agent_session WHERE active = true);"
        ).await;

        let session_sql = match agent_id_filter {
            Some(id) => format!("SELECT * FROM agent_session WHERE agent_id = '{}';", escape(id)),
            None => "SELECT * FROM agent_session WHERE active = true;".to_string(),
        };
        let agents = self.query_one(&session_sql).await.unwrap_or_default();

        let claims_sql = match agent_id_filter {
            Some(id) => format!("SELECT * FROM work_claim WHERE agent_id = '{}' AND active = true;", escape(id)),
            None => "SELECT * FROM work_claim WHERE active = true;".to_string(),
        };
        let claims = self.query_one(&claims_sql).await.unwrap_or_default();

        Ok(CoordSnapshot { agents, claims })
    }

    async fn store_audit(&self, tool: &str, agent_id: &str, details: &serde_json::Value) -> Result<(), CoordError> {
        let escape = |s: &str| escape_surreal(s);
        let safe_details = escape(&details.to_string());
        let query = format!(
            "CREATE mcp_audit SET ts = time::now(), tool = '{}', agent_id = '{}', details = '{}';\
             DELETE mcp_audit WHERE id NOT IN (SELECT VALUE id FROM mcp_audit ORDER BY ts DESC LIMIT 10000);",
            escape(tool), escape(agent_id), safe_details,
        );
        let _ = self.query(&query).await;
        Ok(())
    }

    async fn query_audit(&self, tool_filter: Option<&str>, agent_filter: Option<&str>, limit: u32) -> Result<Vec<serde_json::Value>, CoordError> {
        let escape = |s: &str| escape_surreal(s);
        let limit = limit.min(100);

        let mut conditions = Vec::new();
        if let Some(tool) = tool_filter {
            conditions.push(format!("tool = '{}'", escape(tool)));
        }
        if let Some(agent) = agent_filter {
            conditions.push(format!("agent_id = '{}'", escape(agent)));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let query = format!(
            "SELECT * FROM mcp_audit{} ORDER BY ts DESC LIMIT {};",
            where_clause, limit
        );

        self.query_one(&query).await
            .map_err(|e| CoordError::StorageFailed(format!("Audit query failed: {}", e)))
    }

    async fn heartbeat(&self, agent_id: &str) -> Result<(), CoordError> {
        let sql = format!(
            "UPDATE agent_session:`{}` SET last_seen = time::now();",
            sanitize_record_id(agent_id)
        );
        let _ = self.query_one(&sql).await;
        Ok(())
    }

    async fn deactivate_agent(&self, agent_id: &str) -> Result<(), CoordError> {
        let sql = format!(
            "UPDATE agent_session:`{}` SET active = false, last_seen = time::now();",
            sanitize_record_id(agent_id)
        );
        let _ = self.query_one(&sql).await;
        Ok(())
    }
}
```

- [ ] **Step 3: Build + test**

Run: `source ~/.cargo/env && cargo build -p cynic-kernel --release && cargo test -p cynic-kernel --release && cargo clippy -p cynic-kernel --release -- -D warnings`

Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/storage_http.rs
git commit -m "refactor(storage): implement CoordPort for SurrealHttpStorage"
```

---

### Task 5: Wire CoordPort into MCP — eliminate raw_db (Fix V3)

**Files:**
- Modify: `cynic-kernel/src/mcp.rs` (replace raw_db with coord: Arc<dyn CoordPort>)

- [ ] **Step 1: Update imports in `mcp.rs`**

Replace line 27:
```rust
use crate::storage_http::SurrealHttpStorage;
```
with:
```rust
use crate::coord_port::{CoordPort, ClaimResult};
```

- [ ] **Step 2: Update CynicMcp struct**

Replace line 118:
```rust
    raw_db: Option<Arc<SurrealHttpStorage>>,
```
with:
```rust
    coord: Arc<dyn CoordPort>,
```

- [ ] **Step 3: Update CynicMcp::new constructor**

Replace line 128:
```rust
        raw_db: Option<Arc<SurrealHttpStorage>>,
```
with:
```rust
        coord: Arc<dyn CoordPort>,
```

And update the struct initialization to use `coord` instead of `raw_db`:
```rust
        Self {
            judge,
            storage,
            coord,
            usage,
            tool_router: Self::tool_router(),
        }
```

- [ ] **Step 4: Update audit helper method**

Replace the `audit` method (lines 653-665) with:
```rust
    async fn audit(&self, tool_name: &str, agent_id: &str, details: &serde_json::Value) {
        let _ = self.coord.store_audit(tool_name, agent_id, details).await;
    }
```

- [ ] **Step 5: Update cynic_audit_query tool**

Replace the body of `cynic_audit_query` (lines 389-429) with:
```rust
    async fn cynic_audit_query(
        &self,
        params: Parameters<AuditQueryParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let limit = p.limit.unwrap_or(20).min(100);

        match self.coord.query_audit(p.tool.as_deref(), p.agent_id.as_deref(), limit).await {
            Ok(results) => Ok(CallToolResult::success(vec![
                Content::text(serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".into()))
            ])),
            Err(e) => Ok(CallToolResult::success(vec![
                Content::text(format!("Audit query failed: {}", e))
            ])),
        }
    }
```

- [ ] **Step 6: Update cynic_coord_register tool**

Replace the body. Remove all direct SQL. Use `self.coord.register_agent()`:
```rust
    async fn cynic_coord_register(
        &self,
        params: Parameters<RegisterParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let agent_type = p.agent_type.unwrap_or_else(|| "unknown".into());

        self.coord.register_agent(&p.agent_id, &agent_type, &p.intent).await
            .map_err(|e| McpError::internal_error(format!("Register failed: {}", e), None))?;

        self.audit("cynic_coord_register", &p.agent_id, &serde_json::json!({
            "intent": p.intent, "agent_type": agent_type,
        })).await;

        Ok(CallToolResult::success(vec![
            Content::text(format!("Agent '{}' registered. Intent: {}. Heartbeat refreshed on every MCP call.", p.agent_id, p.intent))
        ]))
    }
```

- [ ] **Step 7: Update cynic_coord_claim tool**

Replace with CoordPort delegation:
```rust
    async fn cynic_coord_claim(
        &self,
        params: Parameters<ClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let claim_type = p.claim_type.unwrap_or_else(|| "file".into());

        match self.coord.claim(&p.agent_id, &p.target, &claim_type).await
            .map_err(|e| McpError::internal_error(format!("Claim failed: {}", e), None))?
        {
            ClaimResult::Conflict(infos) => {
                let conflict_info: Vec<String> = infos.iter()
                    .map(|c| format!("{} (since {})", c.agent_id, c.claimed_at))
                    .collect();
                Ok(CallToolResult::success(vec![
                    Content::text(format!("CONFLICT: '{}' already claimed by: {}. Coordinate before proceeding.",
                        p.target, conflict_info.join(", ")))
                ]))
            }
            ClaimResult::Claimed => {
                self.audit("cynic_coord_claim", &p.agent_id, &serde_json::json!({
                    "target": p.target, "claim_type": claim_type,
                })).await;

                Ok(CallToolResult::success(vec![
                    Content::text(format!("Claimed '{}' ({}) for agent '{}'.", p.target, claim_type, p.agent_id))
                ]))
            }
        }
    }
```

- [ ] **Step 8: Update cynic_coord_release tool**

Replace with CoordPort delegation:
```rust
    async fn cynic_coord_release(
        &self,
        params: Parameters<ReleaseParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;

        let desc = self.coord.release(&p.agent_id, p.target.as_deref()).await
            .map_err(|e| McpError::internal_error(format!("Release failed: {}", e), None))?;

        self.audit("cynic_coord_release", &p.agent_id, &serde_json::json!({
            "target": p.target,
        })).await;

        Ok(CallToolResult::success(vec![Content::text(desc)]))
    }
```

- [ ] **Step 9: Update cynic_coord_who tool**

Replace with CoordPort delegation:
```rust
    async fn cynic_coord_who(
        &self,
        params: Parameters<WhoParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;

        let snapshot = self.coord.who(p.agent_id.as_deref()).await
            .map_err(|e| McpError::internal_error(format!("Who failed: {}", e), None))?;

        let response = serde_json::json!({
            "active_agents": snapshot.agents.len(),
            "active_claims": snapshot.claims.len(),
            "agents": snapshot.agents,
            "claims": snapshot.claims,
        });

        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string_pretty(&response).unwrap_or_else(|_| "{}".into()))
        ]))
    }
```

- [ ] **Step 10: Build + test + clippy**

Run: `source ~/.cargo/env && cargo build -p cynic-kernel --release && cargo test -p cynic-kernel --release && cargo clippy -p cynic-kernel --release -- -D warnings`

Expected: Compile errors because `main.rs` still passes `raw_db` to `CynicMcp::new()`. This is expected — Task 6 fixes it.

- [ ] **Step 11: Do NOT commit yet** — main.rs wiring is Task 6.

---

### Task 6: Wire CoordPort into REST + main.rs — eliminate all raw_db (Fix V2)

**Files:**
- Modify: `cynic-kernel/src/rest.rs` (replace raw_db with coord in AppState)
- Modify: `cynic-kernel/src/main.rs` (create and inject Arc<dyn CoordPort>)

- [ ] **Step 1: Update AppState in `rest.rs`**

In the `AppState` struct, replace:
```rust
    pub raw_db: Option<Arc<storage_http::SurrealHttpStorage>>,
```
with:
```rust
    pub coord: Arc<dyn crate::coord_port::CoordPort>,
```

Remove the import:
```rust
use crate::storage_http;
```
(only if no other usages remain — check first; the `storage_http` import may also be removed from the file header imports if only `raw_db` used it.)

- [ ] **Step 2: Update REST handlers that use `raw_db`**

Search for all uses of `state.raw_db` in `rest.rs` and replace with `state.coord`. These are in the agents handler and any audit-related endpoints.

For the agents handler, replace the raw SQL call with:
```rust
let snapshot = state.coord.who(None).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Coord error: {}", e)))?;
// Format snapshot into the response
```

- [ ] **Step 3: Update `main.rs` — create CoordPort and inject it**

Replace the old `raw_db` wiring. After creating `storage_port` and `raw_db`:

```rust
    // Create CoordPort — delegates to SurrealHttpStorage or NullCoord
    let coord: Arc<dyn coord_port::CoordPort> = match &raw_db {
        Some(s) => Arc::clone(s) as Arc<dyn coord_port::CoordPort>,
        None => Arc::new(coord_port::NullCoord),
    };
```

Update AppState creation:
```rust
    let rest_state = Arc::new(rest::AppState {
        judge: Arc::clone(&judge),
        storage: Arc::clone(&storage_port),
        coord: Arc::clone(&coord),
        usage: Arc::clone(&usage_tracker),
        api_key,
        rate_limiter: rest::RateLimiter::new(30),
        judge_limiter: rest::RateLimiter::new(10),
    });
```

Update MCP creation:
```rust
    let mcp_server = mcp::CynicMcp::new(
        Arc::clone(&judge),
        Arc::clone(&storage_port),
        Arc::clone(&coord),
        Arc::clone(&usage_tracker),
    );
```

**Note:** After this change, `raw_db` is only used to create `storage_port` and `coord`. It is no longer passed to any adapter.

- [ ] **Step 4: Verify no remaining references to `raw_db` in REST or MCP**

Run: `grep -rn "raw_db" cynic-kernel/src/rest.rs cynic-kernel/src/mcp.rs`

Expected: Zero matches.

- [ ] **Step 5: Build + test + clippy**

Run: `source ~/.cargo/env && cargo build -p cynic-kernel --release && cargo test -p cynic-kernel --release && cargo clippy -p cynic-kernel --release -- -D warnings`

Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/mcp.rs cynic-kernel/src/rest.rs cynic-kernel/src/main.rs
git commit -m "refactor(domain): CoordPort wiring — eliminate raw SurrealHttpStorage bypass (fix V2+V3)"
```

---

## Verification

After all 6 tasks are committed, verify the violations are resolved:

- [ ] **V1 check:** `grep -rn "use crate::rest::DogUsageTracker" cynic-kernel/src/` → zero matches
- [ ] **V4 check:** `grep -rn "Arc<BackendRouter>" cynic-kernel/src/hal.rs` → zero matches
- [ ] **V2+V3 check:** `grep -rn "raw_db" cynic-kernel/src/rest.rs cynic-kernel/src/mcp.rs` → zero matches
- [ ] **Full suite:** `cargo build -p cynic-kernel --release && cargo test -p cynic-kernel --release && cargo clippy -p cynic-kernel --release -- -D warnings` → all green
- [ ] **E2E:** `/test-chess` — 3 chess positions score correctly (behavioral equivalence)
