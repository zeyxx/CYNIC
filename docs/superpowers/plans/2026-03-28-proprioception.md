# Proprioception + MCP Write Path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the kernel a nervous system — self-observation, system metrics, and agent write path — tagged as v0.7.4.

**Architecture:** Extend the existing introspection loop to probe system metrics via a new `SystemMetricsPort` trait, submit anomalies through `pipeline::run` (real Dog evaluation), and expose `cynic_observe` as an MCP tool mirroring REST `/observe`. No new crystallization path — everything through the existing pipeline.

**Tech Stack:** Rust, sysinfo 0.37 (already dep), tokio, rmcp, axum

**Spec:** `docs/superpowers/specs/2026-03-28-proprioception-design.md`

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Modify | `cynic-kernel/src/main.rs` | Add `--version` flag (line 14), pass new Arcs to `spawn_introspection` (line 465) |
| Modify | `cynic-kernel/src/introspection.rs` | Accept PipelineDeps components + SystemMetricsPort, replace observe_crystal with pipeline::run, add system metric checks |
| Modify | `cynic-kernel/src/infra/tasks.rs:298-347` | Expand `spawn_introspection` params, raise timeout 30s→180s |
| Modify | `cynic-kernel/src/pipeline.rs:439-446` | Skip `store_crystal_embedding` for domain="cynic-internal" |
| Modify | `cynic-kernel/src/domain/mod.rs` | Add `pub mod system_metrics;` |
| Modify | `cynic-kernel/src/infra/mod.rs` | Add `pub mod system_metrics;` |
| Modify | `cynic-kernel/src/domain/ccm.rs` | Receive `infer_domain()` function + tests from observe.rs, add infra mappings |
| Modify | `cynic-kernel/src/api/rest/observe.rs` | Remove `infer_domain()`, import from domain::ccm |
| Modify | `cynic-kernel/src/api/mcp/mod.rs` | Add `cynic_observe` tool |
| Create | `cynic-kernel/src/domain/system_metrics.rs` | `SystemMetricsPort` trait, `SystemSnapshot`, `SystemMetricsError` |
| Create | `cynic-kernel/src/infra/system_metrics.rs` | `SysinfoMetrics` adapter implementing `SystemMetricsPort` |

---

## Task 1: --version flag

**Files:**
- Modify: `cynic-kernel/src/main.rs:13-14`

- [ ] **Step 1: Add --version flag after existing arg parsing**

In `main.rs`, after line 14 (`let mcp_mode = ...`), add:

```rust
    if std::env::args().any(|a| a == "--version") {
        println!("cynic-kernel {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
```

- [ ] **Step 2: Verify it works**

Run: `cd cynic-kernel && cargo run -- --version`
Expected: `cynic-kernel 0.7.3`

- [ ] **Step 3: Verify normal boot still works**

Run: `cd cynic-kernel && cargo build`
Expected: compiles clean

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/main.rs
git commit -m "feat(kernel): add --version flag — binary self-identification (L1)"
```

---

## Task 2: Anti-contamination gate in pipeline

**Files:**
- Modify: `cynic-kernel/src/pipeline.rs:439-446`

This must come before the introspection changes because it's a prerequisite for safe self-observation crystal storage.

- [ ] **Step 1: Add the KNN exclusion gate**

In `pipeline.rs`, replace lines 439-446:

```rust
    // Anti-contamination: cynic-internal crystals are excluded from KNN index
    // to prevent self-referential noise. Crystal persists (state machine works)
    // but is invisible to semantic search. See introspection.rs module doc.
    if domain != "cynic-internal" {
        if let Some(emb) = stimulus_embedding
            && let Err(e) = deps
                .storage
                .store_crystal_embedding(&crystal_id, &emb.vector)
                .await
        {
            tracing::warn!(phase = "crystal_embed", crystal_id = %crystal_id, error = %e, "failed to store crystal embedding");
        }
    } else {
        tracing::info!(phase = "crystal_embed", crystal_id = %crystal_id, "cynic-internal domain — KNN embedding skipped (anti-contamination)");
    }
```

- [ ] **Step 2: Build and test**

Run: `/build`
Expected: all tests pass, clippy clean

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/src/pipeline.rs
git commit -m "fix(pipeline): skip KNN embedding for cynic-internal domain — anti-contamination (L2)"
```

---

## Task 3: SystemMetricsPort trait

**Files:**
- Create: `cynic-kernel/src/domain/system_metrics.rs`
- Modify: `cynic-kernel/src/domain/mod.rs`

- [ ] **Step 1: Create the port trait file**

Create `cynic-kernel/src/domain/system_metrics.rs`:

```rust
//! Port trait for system-level metrics (CPU, RAM, disk).
//! Domain-pure: no knowledge of sysinfo or any specific implementation.

use async_trait::async_trait;
use std::fmt;

/// Point-in-time snapshot of system resource usage.
#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub cpu_usage_percent: f64,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub disk_available_gb: f64,
    pub disk_total_gb: f64,
    pub load_average_1m: f64,
    pub uptime_seconds: u64,
    pub created_at: String,
}

impl SystemSnapshot {
    /// Compact string for observation context (fits 200-char limit).
    pub fn to_compact(&self) -> String {
        let uptime_display = if self.uptime_seconds >= 86400 {
            format!("{}d", self.uptime_seconds / 86400)
        } else if self.uptime_seconds >= 3600 {
            format!("{}h", self.uptime_seconds / 3600)
        } else {
            format!("{}m", self.uptime_seconds / 60)
        };
        format!(
            "cpu:{:.1}% mem:{:.1}/{:.1}GB disk:{:.0}/{:.0}GB load:{:.2} up:{}",
            self.cpu_usage_percent,
            self.memory_used_gb,
            self.memory_total_gb,
            self.disk_available_gb,
            self.disk_total_gb,
            self.load_average_1m,
            uptime_display,
        )
    }
}

#[derive(Debug)]
pub struct SystemMetricsError(pub String);

impl fmt::Display for SystemMetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "system metrics error: {}", self.0)
    }
}

impl std::error::Error for SystemMetricsError {}

/// Driven port for system-level resource sensing.
/// Implementations: SysinfoMetrics (production), NullMetrics (tests).
#[async_trait]
pub trait SystemMetricsPort: Send + Sync {
    async fn snapshot(&self) -> Result<SystemSnapshot, SystemMetricsError>;
}

/// No-op implementation for tests and environments without system access.
/// Used by introspection tests and as fallback when system probing is unavailable.
#[allow(dead_code)] // Used in tests and as potential runtime fallback
pub struct NullSystemMetrics;

#[async_trait]
impl SystemMetricsPort for NullSystemMetrics {
    async fn snapshot(&self) -> Result<SystemSnapshot, SystemMetricsError> {
        Err(SystemMetricsError("NullSystemMetrics — no system access".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_format_fits_200_chars() {
        let snap = SystemSnapshot {
            cpu_usage_percent: 42.1,
            memory_used_gb: 11.2,
            memory_total_gb: 15.5,
            disk_available_gb: 120.0,
            disk_total_gb: 500.0,
            load_average_1m: 0.82,
            uptime_seconds: 3 * 86400 + 7200,
            created_at: "2026-03-28T12:00:00Z".into(),
        };
        let compact = snap.to_compact();
        assert!(compact.len() <= 200, "compact was {} chars: {}", compact.len(), compact);
        assert!(compact.contains("cpu:42.1%"));
        assert!(compact.contains("mem:11.2/15.5GB"));
        assert!(compact.contains("up:3d"));
    }

    #[test]
    fn error_display_and_error_trait() {
        let e = SystemMetricsError("test".into());
        assert!(e.to_string().contains("test"));
        // K7: Display implies Error — this must compile
        let _: &dyn std::error::Error = &e;
    }

    #[tokio::test]
    async fn null_metrics_returns_err() {
        let null = NullSystemMetrics;
        assert!(null.snapshot().await.is_err());
    }
}
```

- [ ] **Step 2: Wire in domain/mod.rs**

Add to `cynic-kernel/src/domain/mod.rs` after line 16 (`pub mod summarization;`):

```rust
pub mod system_metrics;
```

- [ ] **Step 3: Build and test**

Run: `/build`
Expected: all tests pass (including the 3 new ones), clippy clean

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/domain/system_metrics.rs cynic-kernel/src/domain/mod.rs
git commit -m "feat(domain): add SystemMetricsPort trait — proprioception contract (L3)"
```

---

## Task 4: SysinfoMetrics adapter

**Files:**
- Create: `cynic-kernel/src/infra/system_metrics.rs`
- Modify: `cynic-kernel/src/infra/mod.rs`

- [ ] **Step 1: Create the adapter**

Create `cynic-kernel/src/infra/system_metrics.rs`:

```rust
//! sysinfo-based adapter for SystemMetricsPort.
//! Per-tick System::new() + targeted refresh — same proven pattern as probe/hardware.rs.
//! All sysinfo calls run inside spawn_blocking (they are synchronous).

use crate::domain::system_metrics::{SystemMetricsError, SystemMetricsPort, SystemSnapshot};
use async_trait::async_trait;

pub struct SysinfoMetrics;

impl SysinfoMetrics {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SystemMetricsPort for SysinfoMetrics {
    async fn snapshot(&self) -> Result<SystemSnapshot, SystemMetricsError> {
        tokio::task::spawn_blocking(|| {
            use sysinfo::{Disks, System};

            let mut sys = System::new();
            sys.refresh_memory();
            sys.refresh_cpu_usage(); // First call seeds the measurement

            // CPU usage needs two samples. We do a brief sleep inside spawn_blocking
            // (this is a blocking thread, not async — sleep is safe here).
            std::thread::sleep(std::time::Duration::from_millis(200));
            sys.refresh_cpu_usage();

            let cpu_usage = if sys.cpus().is_empty() {
                0.0
            } else {
                sys.cpus().iter().map(|c| c.cpu_usage() as f64).sum::<f64>()
                    / sys.cpus().len() as f64
            };

            let memory_total_gb = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
            let memory_used_gb = sys.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0);

            let disks = Disks::new_with_refreshed_list();
            let (disk_available, disk_total) = disks.iter().fold((0u64, 0u64), |(a, t), d| {
                (a + d.available_space(), t + d.total_space())
            });
            let disk_available_gb = disk_available as f64 / (1024.0 * 1024.0 * 1024.0);
            let disk_total_gb = disk_total as f64 / (1024.0 * 1024.0 * 1024.0);

            let load_avg = System::load_average();

            Ok(SystemSnapshot {
                cpu_usage_percent: cpu_usage,
                memory_used_gb,
                memory_total_gb,
                disk_available_gb,
                disk_total_gb,
                load_average_1m: load_avg.one,
                uptime_seconds: System::uptime(),
                created_at: chrono::Utc::now().to_rfc3339(),
            })
        })
        .await
        .map_err(|e| SystemMetricsError(format!("spawn_blocking join error: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn snapshot_returns_sane_values() {
        let metrics = SysinfoMetrics::new();
        let snap = metrics.snapshot().await.expect("snapshot should succeed");
        assert!(snap.memory_total_gb > 0.0, "total RAM must be positive");
        assert!(snap.memory_used_gb >= 0.0, "used RAM must be non-negative");
        assert!(snap.memory_used_gb <= snap.memory_total_gb, "used <= total");
        assert!(snap.disk_total_gb > 0.0, "disk total must be positive");
        assert!(snap.uptime_seconds > 0, "uptime must be positive");
        // CPU usage can be 0.0 on first tick — not asserting > 0
        assert!(snap.cpu_usage_percent >= 0.0 && snap.cpu_usage_percent <= 100.0);
    }
}
```

- [ ] **Step 2: Wire in infra/mod.rs**

Add to `cynic-kernel/src/infra/mod.rs` after line 6 (`pub mod tasks;`):

```rust
pub mod system_metrics;
```

- [ ] **Step 3: Build and test**

Run: `/build`
Expected: all tests pass (including snapshot_returns_sane_values), clippy clean

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/infra/system_metrics.rs cynic-kernel/src/infra/mod.rs
git commit -m "feat(infra): add SysinfoMetrics adapter — real system sensing (L4a)"
```

---

## Task 5: Introspection broadened — T8 fix + system metrics

**Files:**
- Modify: `cynic-kernel/src/introspection.rs` (full rewrite of analyze function)
- Modify: `cynic-kernel/src/infra/tasks.rs:298-347` (expand spawn_introspection)
- Modify: `cynic-kernel/src/main.rs:465-472` (pass new Arcs)

This is the largest task — the core of the proprioception deliverable.

- [ ] **Step 1: Update introspection.rs — new signature and system checks**

Rewrite `cynic-kernel/src/introspection.rs`. The key changes:

1. `analyze()` receives the Arcs needed to construct `PipelineDeps<'_>` + `SystemMetricsPort`
2. System metric checks added (memory, CPU, disk)
3. Alerts consolidated into ONE `pipeline::run` call (not one per alert)
4. `observe_crystal(voter_count=0)` replaced with `pipeline::run(domain="cynic-internal")`
5. Raw snapshot stored as observation

New signature:
```rust
pub async fn analyze(
    storage: &dyn StoragePort,
    metrics: &Metrics,
    system_metrics: &dyn SystemMetricsPort,
    judge: &Judge,
    embedding: &dyn EmbeddingPort,
    usage: &Mutex<DogUsageTracker>,
    verdict_cache: &VerdictCache,
    event_tx: Option<&tokio::sync::broadcast::Sender<KernelEvent>>,
) -> Vec<Alert>
```

Inside the function:
- Run existing 4 checks (dog failure, embedding failure, zero verdicts, storage down)
- Call `system_metrics.snapshot()` — if Ok, run 3 new checks (memory >90%, CPU >80% unless 0.0, disk <10%)
- Store raw snapshot as observation: `storage.store_observation(...)` with `tool="self-probe"`, `domain="infra"`, `context=snapshot.to_compact()`
- If any alerts: concatenate into single content string (bounded 2000 chars), construct `PipelineDeps`, then:
  ```rust
  let dogs: Vec<String> = vec!["deterministic-dog".into(), "gemini-flash".into()];
  let _ = crate::pipeline::run(
      content,
      None,                              // context
      Some("cynic-internal".to_string()), // domain
      Some(dogs.as_slice()),              // dogs_filter — NOTE: must be &[String], not &[&str]
      false,                              // inject_crystals=false
      &deps,
  ).await;
  ```
- Remove the old `observe_crystal(voter_count=0)` call entirely

- [ ] **Step 2: Update infra/tasks.rs — expand spawn_introspection**

Modify `spawn_introspection` at line 298 to accept additional parameters:

```rust
pub fn spawn_introspection(
    storage: Arc<dyn StoragePort>,
    metrics: Arc<Metrics>,
    system_metrics: Arc<dyn SystemMetricsPort>,
    judge: Arc<Judge>,
    embedding: Arc<dyn EmbeddingPort>,
    usage: Arc<Mutex<DogUsageTracker>>,
    verdict_cache: Arc<VerdictCache>,
    introspection_alerts: Arc<std::sync::RwLock<Vec<crate::introspection::Alert>>>,
    event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()>
```

Inside the task:
- Change timeout from `30` to `180` seconds (line 322)
- Update the timeout log message from `"30s"` to `"180s"` (line 339) to match the actual value
- Update the `analyze` call to pass all new dependencies
- The analyze function constructs `PipelineDeps<'_>` from borrowed refs to the Arcs

- [ ] **Step 3: Update main.rs — pass new Arcs**

At `main.rs` line 465, expand the `spawn_introspection` call:

```rust
    infra::tasks::spawn_introspection(
        Arc::clone(&storage_port),
        Arc::clone(&metrics),
        system_metrics.clone(),  // NEW
        Arc::clone(&judge),      // NEW
        Arc::clone(&embedding),  // NEW
        Arc::clone(&usage_tracker), // NEW
        Arc::clone(&verdict_cache), // NEW
        Arc::clone(&rest_state.introspection_alerts),
        event_tx.clone(),
        Arc::clone(&task_health),
        shutdown.clone(),
    );
```

Also at Ring 0 (after `probe::run`), create the system_metrics instance:

```rust
    let system_metrics: Arc<dyn crate::domain::system_metrics::SystemMetricsPort> =
        Arc::new(infra::system_metrics::SysinfoMetrics::new());
```

- [ ] **Step 4: Update existing introspection tests**

The existing tests in `introspection.rs` use `analyze(&NullStorage, &metrics)`. Update them to pass `NullSystemMetrics` and the other new deps. Use `NullInfer`-like stubs for Judge/EmbeddingPort (or skip the pipeline::run path in tests by having zero alerts).

For the healthy system test: NullSystemMetrics returns Err → system metrics check is skipped (graceful degradation). Existing behavior preserved.

For the high failure rate test: same — NullSystemMetrics error is tolerated, dog_failure_rate alert still detected.

- [ ] **Step 5: Build and test**

Run: `/build`
Expected: all tests pass, clippy clean

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/introspection.rs cynic-kernel/src/infra/tasks.rs cynic-kernel/src/main.rs
git commit -m "feat(kernel): broadened introspection — T8 fix + system metrics (L2+L4b)"
```

---

## Task 6: Extract infer_domain to domain + add infra mappings

**Files:**
- Modify: `cynic-kernel/src/domain/ccm.rs` (receive function + tests)
- Modify: `cynic-kernel/src/api/rest/observe.rs` (remove function, import from domain)

- [ ] **Step 1: Add infer_domain to domain/ccm.rs with infra mappings**

Add to `cynic-kernel/src/domain/ccm.rs` (at the end, before `#[cfg(test)]`):

```rust
/// Infer domain from file extension or target pattern.
/// Moved from api/rest/observe.rs — this is domain logic (K5 compliance).
pub fn infer_domain(target: Option<&str>, tool: Option<&str>) -> String {
    // Tool-based inference (highest priority)
    if let Some(t) = tool {
        if t == "self-probe" {
            return "infra".to_string();
        }
    }

    let target = match target {
        Some(t) if !t.is_empty() => t,
        _ => return "general".to_string(),
    };

    // Infra target patterns
    if target.starts_with("cynic-") || target.contains("llama-server") || target.contains("surrealdb") {
        return "infra".to_string();
    }

    // Extension-based inference
    target
        .rsplit('.')
        .next()
        .map(|ext| match ext {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" => "python",
            "md" => "docs",
            "toml" | "json" | "yaml" | "yml" => "config",
            "service" | "timer" => "infra",
            _ => "general",
        })
        .unwrap_or("general")
        .to_string()
}
```

- [ ] **Step 2: Add tests for infer_domain in domain/ccm.rs**

Add to the `#[cfg(test)]` module in `domain/ccm.rs`:

```rust
    // ── infer_domain (moved from api/rest/observe.rs) ──

    #[test]
    fn infer_domain_rust() {
        assert_eq!(infer_domain(Some("src/judge.rs"), None), "rust");
    }

    #[test]
    fn infer_domain_typescript() {
        assert_eq!(infer_domain(Some("App.tsx"), None), "typescript");
        assert_eq!(infer_domain(Some("utils.ts"), None), "typescript");
    }

    #[test]
    fn infer_domain_javascript() {
        assert_eq!(infer_domain(Some("index.js"), None), "javascript");
    }

    #[test]
    fn infer_domain_python() {
        assert_eq!(infer_domain(Some("train.py"), None), "python");
    }

    #[test]
    fn infer_domain_config() {
        assert_eq!(infer_domain(Some("Cargo.toml"), None), "config");
        assert_eq!(infer_domain(Some("package.json"), None), "config");
        assert_eq!(infer_domain(Some("config.yaml"), None), "config");
    }

    #[test]
    fn infer_domain_docs() {
        assert_eq!(infer_domain(Some("README.md"), None), "docs");
    }

    #[test]
    fn infer_domain_general_fallback() {
        assert_eq!(infer_domain(Some("binary.wasm"), None), "general");
        assert_eq!(infer_domain(None, None), "general");
        assert_eq!(infer_domain(Some("Makefile"), None), "general");
    }

    // New infra mappings
    #[test]
    fn infer_domain_infra_service() {
        assert_eq!(infer_domain(Some("cynic-kernel.service"), None), "infra");
        assert_eq!(infer_domain(Some("backup.timer"), None), "infra");
    }

    #[test]
    fn infer_domain_infra_targets() {
        assert_eq!(infer_domain(Some("cynic-core"), None), "infra");
        assert_eq!(infer_domain(Some("cynic-gpu"), None), "infra");
        assert_eq!(infer_domain(Some("llama-server"), None), "infra");
        assert_eq!(infer_domain(Some("surrealdb"), None), "infra");
    }

    #[test]
    fn infer_domain_self_probe_tool() {
        assert_eq!(infer_domain(Some("localhost"), Some("self-probe")), "infra");
        assert_eq!(infer_domain(None, Some("self-probe")), "infra");
    }
```

- [ ] **Step 3: Update api/rest/observe.rs to use domain function**

In `cynic-kernel/src/api/rest/observe.rs`:

1. Remove the `infer_domain` function (lines 25-39) and all its tests (lines 113-163)
2. Add import: `use crate::domain::ccm::infer_domain;`
3. Update the call site (line 57-58) to pass both target and tool:
```rust
    let domain = req
        .domain
        .unwrap_or_else(|| infer_domain(req.target.as_deref(), Some(&req.tool)));
```
**IMPORTANT:** This line MUST remain before line 63 (`tool: req.tool`) where `req.tool` is moved. The borrow `&req.tool` at line 58 must precede the move at line 63.

- [ ] **Step 4: Build and test**

Run: `/build`
Expected: all tests pass. Old observe.rs tests replaced by domain/ccm.rs tests.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/domain/ccm.rs cynic-kernel/src/api/rest/observe.rs
git commit -m "refactor(domain): extract infer_domain to domain/ccm — K5 fix + infra mappings (L5a)"
```

---

## Task 7: MCP cynic_observe tool

**Files:**
- Modify: `cynic-kernel/src/api/mcp/mod.rs`

- [ ] **Step 1: Add ObserveParams and cynic_observe tool**

In `api/mcp/mod.rs`, add the params struct near the other param structs:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ObserveParams {
    /// Tool or action name (1-64 chars)
    pub tool: String,
    /// Target file, resource, or entity
    pub target: Option<String>,
    /// Domain classification (auto-inferred from target if omitted)
    pub domain: Option<String>,
    /// Status: ok, warning, error
    pub status: Option<String>,
    /// Additional context (max 200 chars, truncated if longer)
    pub context: Option<String>,
    /// Project identifier
    pub project: Option<String>,
    /// Agent identifier for session tracking
    pub agent_id: Option<String>,
    /// Session identifier for CCM aggregation
    pub session_id: Option<String>,
}
```

Add the tool method inside the `#[tool_router]` impl block:

```rust
    // NOTE: Uses Parameters<T> wrapper — this is the codebase pattern (not #[tool(aggr)])
    #[tool(name = "cynic_observe")]
    async fn cynic_observe(
        &self,
        params: Parameters<ObserveParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let params = params.0;

        // Validate
        if params.tool.is_empty() || params.tool.len() > 64 {
            return Err(McpError::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                "tool must be 1-64 characters",
                None,
            ));
        }

        let domain = params.domain.unwrap_or_else(|| {
            crate::domain::ccm::infer_domain(params.target.as_deref(), Some(&params.tool))
        });

        let agent_id = params.agent_id.clone().unwrap_or_else(|| "unknown".into());
        let tool_name = params.tool.clone();

        let obs = crate::domain::storage::Observation {
            project: params.project.unwrap_or_else(|| "CYNIC".into()),
            agent_id: agent_id.clone(),
            tool: params.tool,
            target: params.target.unwrap_or_default(),
            domain,
            status: params.status.unwrap_or_else(|| "success".into()),
            context: params
                .context
                .map(|c| c.chars().take(200).collect())
                .unwrap_or_default(),
            session_id: params.session_id.unwrap_or_default(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let storage = Arc::clone(&self.storage);
        tokio::spawn(async move {
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                storage.store_observation(&obs),
            )
            .await
            {
                Ok(Err(e)) => tracing::warn!(error = %e, "cynic_observe: store failed"),
                Err(_) => tracing::warn!("cynic_observe: store timed out (5s)"),
                _ => {}
            }
        });

        // NOTE: audit third arg is &serde_json::Value, not &str
        self.audit("cynic_observe", &agent_id, &serde_json::json!({ "tool": tool_name }))
            .await;

        Ok(CallToolResult::success(vec![Content::text(
            r#"{"status":"observed"}"#,
        )]))
    }
```

- [ ] **Step 2: Update the router macro**

Make sure `cynic_observe` is included in the `#[tool_router]` list. Check the existing pattern — each tool method with `#[tool(name = "...")]` is auto-registered by the macro.

- [ ] **Step 3: Add unit test for cynic_observe**

In the `#[cfg(test)]` module of `api/mcp/mod.rs`, add a test following the existing pattern:

```rust
#[test]
fn observe_params_deserialize_minimal() {
    let json = r#"{"tool":"Read"}"#;
    let params: ObserveParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.tool, "Read");
    assert!(params.target.is_none());
    assert!(params.agent_id.is_none());
}

#[test]
fn observe_params_deserialize_full() {
    let json = r#"{"tool":"Edit","target":"src/main.rs","domain":"rust","agent_id":"claude-123","session_id":"s1"}"#;
    let params: ObserveParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.tool, "Edit");
    assert_eq!(params.agent_id.as_deref(), Some("claude-123"));
}
```

- [ ] **Step 4: Build and test**

Run: `/build`
Expected: all tests pass, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/api/mcp/mod.rs
git commit -m "feat(mcp): add cynic_observe tool — agent write path (L5b)"
```

---

## Task 8: Version bump, tag, and deploy

**Files:**
- Modify: `cynic-kernel/Cargo.toml` (version field)

- [ ] **Step 1: Final build verification**

Run: `/build`
Expected: all tests pass, zero warnings

- [ ] **Step 2: Bump version**

In `cynic-kernel/Cargo.toml`, change `version = "0.7.3"` to `version = "0.7.4"`.

- [ ] **Step 3: Verify --version reflects new version**

Run: `cd cynic-kernel && cargo run -- --version`
Expected: `cynic-kernel 0.7.4`

- [ ] **Step 4: Commit and tag**

```bash
git add cynic-kernel/Cargo.toml
git commit -m "chore: bump version to v0.7.4 — proprioception + MCP write path"
git tag v0.7.4
```

- [ ] **Step 5: Deploy**

Run: `/deploy`

- [ ] **Step 6: Verify deployment**

Run: `/status`
Expected: kernel running v0.7.4, Dogs healthy, 11 MCP tools

---

## Summary

| Task | Deliverable | Files | Est. |
|------|-------------|-------|------|
| 1 | --version flag | main.rs | 2 min |
| 2 | Anti-contamination gate | pipeline.rs | 5 min |
| 3 | SystemMetricsPort trait | domain/system_metrics.rs, domain/mod.rs | 5 min |
| 4 | SysinfoMetrics adapter | infra/system_metrics.rs, infra/mod.rs | 5 min |
| 5 | Introspection T8 fix + broadening | introspection.rs, infra/tasks.rs, main.rs | 15 min |
| 6 | Extract infer_domain + infra mappings | domain/ccm.rs, api/rest/observe.rs | 10 min |
| 7 | MCP cynic_observe | api/mcp/mod.rs | 10 min |
| 8 | Version bump + deploy | Cargo.toml | 5 min |
