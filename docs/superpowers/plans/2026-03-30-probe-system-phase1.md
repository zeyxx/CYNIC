# Probe System Phase 1 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give CYNIC proprioception — the kernel senses its own resources and backup health via a `Probe` trait system, persists snapshots to SurrealDB, and exposes them in `/health`.

**Architecture:** Domain-pure `Probe` trait with typed `ProbeDetails` enum. `ProbeScheduler` in infra runs probes at individual intervals, stores `EnvironmentSnapshot` in AppState and SurrealDB. `/health` reads the snapshot. Phase 1 adds new code only — does NOT touch health_loop, system_metrics, or introspection.

**Tech Stack:** Rust 1.94+, async-trait (already in Cargo.toml), sysinfo 0.37 (already in Cargo.toml), tokio, axum, SurrealDB HTTP.

**Spec:** `docs/reference/PROBE-SYSTEM-SPEC.md`

**Build command:** `make check` (must pass after every task)

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `cynic-kernel/src/domain/probe.rs` | Probe trait, ProbeResult, ProbeDetails, ProbeStatus, EnvironmentSnapshot, NullProbe |
| Modify | `cynic-kernel/src/domain/mod.rs:17` | Add `pub mod probe;` |
| Modify | `cynic-kernel/src/domain/storage.rs` | 3 new methods with default no-ops (insert before trait closing `}`) |
| Create | `cynic-kernel/src/infra/probes/mod.rs` | ProbeScheduler |
| Create | `cynic-kernel/src/infra/probes/resource.rs` | ResourceProbe (CPU/RAM/disk via sysinfo) |
| Create | `cynic-kernel/src/infra/probes/backup.rs` | BackupProbe (stat backup dir, check age) |
| Modify | `cynic-kernel/src/infra/mod.rs:7` | Add `pub mod probes;` |
| Modify | `cynic-kernel/src/infra/task_health.rs` | Add `probe_scheduler` field + touch + snapshot entry |
| Modify | `cynic-kernel/src/infra/tasks.rs:467` | Add `spawn_probe_scheduler()` |
| Modify | `cynic-kernel/src/storage/surreal.rs:870` | Implement store/query/cleanup infra_snapshot |
| Modify | `infra/surrealdb/schema.surql:141` | Add `infra_snapshot` table |
| Modify | `cynic-kernel/src/api/rest/types.rs:57` | Add `environment` field to AppState |
| Modify | `cynic-kernel/src/api/rest/health.rs:163` | Add `environment` to /health response |
| Modify | `cynic-kernel/src/main.rs:510` | Wire spawn_probe_scheduler + AppState field |

---

### Task 1: Domain Contract — Probe trait + types

**Files:**
- Create: `cynic-kernel/src/domain/probe.rs`
- Modify: `cynic-kernel/src/domain/mod.rs`

- [ ] **Step 1: Create `domain/probe.rs` with trait + types + NullProbe**

Write the full domain contract as specified in `PROBE-SYSTEM-SPEC.md` Domain Contract section:
- `Probe` trait with `#[async_trait]`, methods: `name()`, `interval()`, `sense()`
- `ProbeResult`, `ProbeStatus`, `ProbeDetails` enum (Resource, Backup, DogHealth, Network, OsCapability, Empty variants)
- All detail structs: `ResourceDetails`, `BackupDetails`, `DogHealthDetails`, `NetworkDetails`, `NetInterfaceInfo`, `PeerInfo`, `OsCapDetails`
- `EnvironmentSnapshot` with `worst_status()` helper
- `ProbeError` with `Internal` variant
- `NullProbe` struct + impl
- `#[cfg(test)] mod tests` with: `null_probe_returns_unavailable`, `worst_status_logic`, `probe_status_ordering`

- [ ] **Step 2: Register module in `domain/mod.rs`**

Add `pub mod probe;` after the last module declaration.

- [ ] **Step 3: Run `make check` — verify compilation + tests pass**

Run: `make check`
Expected: All existing tests pass + 3 new tests in `domain::probe::tests`

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/domain/probe.rs cynic-kernel/src/domain/mod.rs
git commit -m "feat(probe): domain contract — Probe trait, typed ProbeDetails, NullProbe"
```

---

### Task 2: StoragePort Extension — 3 new methods

**Files:**
- Modify: `cynic-kernel/src/domain/storage.rs`

- [ ] **Step 1: Add 3 methods to `StoragePort` trait with default no-ops**

Insert before the closing `}` of the trait (line ~319). All have default implementations so `NullStorage` and test adapters are unchanged:

```rust
    // ── Infrastructure snapshots (probe system) ──────────────

    async fn store_infra_snapshot(
        &self,
        _snap: &crate::domain::probe::EnvironmentSnapshot,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    async fn list_infra_snapshots(
        &self,
        _hours: u32,
    ) -> Result<Vec<crate::domain::probe::EnvironmentSnapshot>, StorageError> {
        Ok(vec![])
    }

    async fn cleanup_infra_snapshots(
        &self,
        _older_than_days: u32,
    ) -> Result<u64, StorageError> {
        Ok(0)
    }
```

- [ ] **Step 2: Run `make check` — verify nothing breaks**

Run: `make check`
Expected: All tests pass (defaults mean no impl changes needed anywhere)

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/src/domain/storage.rs
git commit -m "feat(probe): StoragePort extension — 3 infra_snapshot methods with default no-ops"
```

---

### Task 3: TaskHealth Extension — probe_scheduler tracking

**Files:**
- Modify: `cynic-kernel/src/infra/task_health.rs`

- [ ] **Step 1: Write test for probe_scheduler tracking**

Add to the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn probe_scheduler_visible_in_snapshot() {
        let th = TaskHealth::new();
        assert!(th.snapshot().iter().any(|s| s.name == "probe_scheduler"));
        th.touch_probe_scheduler();
        let snap = th.snapshot();
        let ps = snap.iter().find(|s| s.name == "probe_scheduler").expect("probe_scheduler");
        assert_eq!(ps.status, "ok");
    }
```

- [ ] **Step 2: Run test — verify it fails**

Run: `cargo test -p cynic-kernel --lib task_health::tests::probe_scheduler_visible -- --release`
Expected: FAIL — `touch_probe_scheduler` does not exist

- [ ] **Step 3: Implement — add field, init, touch, snapshot entry**

Add `probe_scheduler: AtomicU64` field to `TaskHealth` struct.
Add `probe_scheduler: AtomicU64::new(0)` to `new()`.
Add `touch_probe_scheduler` method (same pattern as `touch_coord_expiry`).
Add `TaskSnapshot::new("probe_scheduler", self.probe_scheduler.load(...), now, 20, None)` to the `snapshot()` vec. Expected interval: 20s (scheduler wakes every 10s, so 2x = 20s stale threshold).

- [ ] **Step 4: Run `make check`**

Expected: All tests pass including the new one.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/infra/task_health.rs
git commit -m "feat(probe): TaskHealth tracks probe_scheduler background task"
```

---

### Task 4: ResourceProbe — CPU/RAM/disk sensing

**Files:**
- Create: `cynic-kernel/src/infra/probes/resource.rs`
- Create: `cynic-kernel/src/infra/probes/mod.rs` (partial — just re-export for now)
- Modify: `cynic-kernel/src/infra/mod.rs`

- [ ] **Step 1: Create `infra/probes/mod.rs` with resource module only**

Note: Do NOT add `pub mod backup` yet — `backup.rs` does not exist until Task 5.

```rust
pub mod resource;

// Re-exports
pub use resource::ResourceProbe;
```

- [ ] **Step 2: Register `probes` module in `infra/mod.rs`**

Add `pub mod probes;` after the last line.

- [ ] **Step 3: Write ResourceProbe with tests**

Create `infra/probes/resource.rs`:
- `ResourceProbe` struct (no fields — stateless like `SysinfoMetrics`)
- `#[async_trait] impl Probe for ResourceProbe` — uses `tokio::task::spawn_blocking` with sysinfo (same pattern as `system_metrics.rs`): `System::new()`, `refresh_memory()`, `refresh_cpu_usage()`, 200ms sleep, second `refresh_cpu_usage()`, `Disks::new_with_refreshed_list()`, `System::load_average()`, `System::uptime()`
- Returns `ProbeResult { details: ProbeDetails::Resource(ResourceDetails { ... }) }`
- `name()` returns `"resource"`, `interval()` returns `Duration::from_secs(300)` (5min)
- Test: `resource_probe_returns_ok` — calls `sense()`, asserts `status == Ok`, asserts `details` is `ProbeDetails::Resource(...)` with non-None `memory_total_gb`

- [ ] **Step 4: Run `make check`**

Expected: All tests pass including new ResourceProbe test.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/infra/probes/ cynic-kernel/src/infra/mod.rs
git commit -m "feat(probe): ResourceProbe — CPU/RAM/disk sensing via sysinfo"
```

---

### Task 5: BackupProbe — backup freshness sensing

**Files:**
- Create: `cynic-kernel/src/infra/probes/backup.rs`

- [ ] **Step 1: Add backup module to `infra/probes/mod.rs`**

Add to `infra/probes/mod.rs`:
```rust
pub mod backup;
pub use backup::BackupProbe;
```

- [ ] **Step 2: Write BackupProbe with tests**

Create `infra/probes/backup.rs`:
- `BackupProbe` struct with `backup_dir: PathBuf`
- Constructor: `BackupProbe::new(backup_dir: PathBuf)`
- `#[async_trait] impl Probe for BackupProbe`:
  - `name()` → `"backup"`, `interval()` → `Duration::from_secs(3600)` (1h)
  - `sense()`: `tokio::fs::read_dir` on `backup_dir`, find most recent `.surql.gz` file by metadata modified time, compute age in hours, count files, sum sizes
  - On `NotFound` → `ProbeStatus::Unavailable`
  - On `PermissionDenied` → `ProbeStatus::Denied`
  - Returns `ProbeDetails::Backup(BackupDetails { ... })`
- Tests:
  - `backup_probe_missing_dir_returns_unavailable` — point at `/tmp/nonexistent-cynic-test-dir`, assert Unavailable
  - `backup_probe_empty_dir_returns_degraded` — create tmpdir, assert Degraded (no backups found)
  - `backup_probe_with_file_returns_ok` — create tmpdir with a `.surql.gz` file, assert Ok + age < 1 hour

- [ ] **Step 3: Run `make check`**

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/infra/probes/backup.rs cynic-kernel/src/infra/probes/mod.rs
git commit -m "feat(probe): BackupProbe — backup freshness and health sensing"
```

---

### Task 6: ProbeScheduler — per-probe interval scheduling

**Files:**
- Modify: `cynic-kernel/src/infra/probes/mod.rs`

- [ ] **Step 1: Write ProbeScheduler with tests**

Add to `infra/probes/mod.rs`:
- `ProbeScheduler` struct: `probes: Vec<ProbeSlot>` where `ProbeSlot { probe: Arc<dyn Probe>, last_fired: Instant }`
- `ProbeScheduler::new(probes: Vec<Arc<dyn Probe>>) -> Self`
- `ProbeScheduler::tick(&mut self) -> Option<EnvironmentSnapshot>`:
  - Check each probe: `now - last_fired >= probe.interval()`
  - Collect due probes, fire in parallel with `futures_util::future::join_all` + per-probe `tokio::time::timeout` (min(2 * interval, 30s))
  - For `Err(ProbeError::Internal)`: log error, exclude from results
  - For timeout: create `ProbeResult { status: Unavailable, details: Empty }`
  - If no probes were due: return `None`
  - Assemble `EnvironmentSnapshot` with `overall = worst_status()`
- Tests:
  - `scheduler_fires_due_probes` — create scheduler with NullProbe(interval=0s), tick once, assert Some(snapshot) with 1 probe result
  - `scheduler_skips_not_due_probes` — create scheduler with NullProbe(interval=3600s), tick once immediately, assert None (first tick after initial fire)
  - `scheduler_handles_probe_error` — create a FaultyProbe that returns Err, tick, assert snapshot still returned (without the faulty probe)

Define a `TestProbe` helper in the test module:

```rust
#[cfg(test)]
struct TestProbe {
    name: &'static str,
    interval: Duration,
    result: Result<ProbeResult, ProbeError>,
}

#[cfg(test)]
#[async_trait]
impl Probe for TestProbe {
    fn name(&self) -> &str { self.name }
    fn interval(&self) -> Duration { self.interval }
    async fn sense(&self) -> Result<ProbeResult, ProbeError> { self.result.clone() }
}
```

- [ ] **Step 2: Run `make check`**

Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/src/infra/probes/mod.rs
git commit -m "feat(probe): ProbeScheduler — per-probe interval scheduling with parallel fan-out"
```

---

### Task 7: SurrealDB — schema + adapter

**Files:**
- Modify: `infra/surrealdb/schema.surql`
- Modify: `cynic-kernel/src/storage/surreal.rs`

- [ ] **Step 1: Add `infra_snapshot` table to schema.surql**

Append to end of file:

```sql
-- ── Infrastructure snapshots (probe system) ──────────────────
DEFINE TABLE IF NOT EXISTS infra_snapshot SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS ts ON infra_snapshot TYPE datetime;
DEFINE FIELD IF NOT EXISTS overall ON infra_snapshot TYPE string;
DEFINE FIELD IF NOT EXISTS probes ON infra_snapshot TYPE array;
DEFINE FIELD IF NOT EXISTS probes.* ON infra_snapshot TYPE object;
DEFINE INDEX IF NOT EXISTS idx_infra_ts ON infra_snapshot FIELDS ts;
```

- [ ] **Step 2: Implement 3 StoragePort methods in `SurrealHttpStorage`**

Insert before the closing `}` of the `impl StoragePort for SurrealHttpStorage` block:
- `store_infra_snapshot`: INSERT with `ts`, `overall` (string), `probes` (JSON array)
- `list_infra_snapshots(hours)`: SELECT from `infra_snapshot` WHERE `ts > time::now() - {hours}h` ORDER BY `ts` DESC LIMIT 100
- `cleanup_infra_snapshots(days)`: DELETE from `infra_snapshot` WHERE `ts < time::now() - {days}d`, return count

Follow existing SQL patterns (parameterized queries, `escape_surreal` for strings, `self.query_one`/`self.query_raw`).

- [ ] **Step 3: Run `make check`**

Expected: All tests pass. (Integration tests with real SurrealDB will be added as `#[ignore]` in a later step.)

- [ ] **Step 4: Commit**

```bash
git add infra/surrealdb/schema.surql cynic-kernel/src/storage/surreal.rs
git commit -m "feat(probe): SurrealDB infra_snapshot — store, query, cleanup"
```

---

### Task 8: Background task — spawn_probe_scheduler

**Files:**
- Modify: `cynic-kernel/src/infra/tasks.rs`

- [ ] **Step 1: Write `spawn_probe_scheduler` function**

Add after the last spawn function, before `#[cfg(test)]`:

```rust
pub fn spawn_probe_scheduler(
    probes: Vec<Arc<dyn crate::domain::probe::Probe>>,
    storage: Arc<dyn StoragePort>,
    environment: Arc<std::sync::RwLock<Option<crate::domain::probe::EnvironmentSnapshot>>>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    // ... follows the canonical spawn pattern:
    // interval 10s, skip first tick, select! on shutdown,
    // call scheduler.tick(), if Some(snap): persist + update environment + touch task_health
    // Every 100 ticks: cleanup_infra_snapshots(7)
    //
    // K6 NOTE: No outer timeout on scheduler.tick() needed — K6 is satisfied
    // by the per-probe timeouts inside ProbeScheduler::tick(). Each probe's
    // .await is wrapped in tokio::time::timeout(min(2*interval, 30s)).
}
```

- [ ] **Step 2: Write test**

Add to the `#[cfg(test)] mod tests` in tasks.rs:

```rust
    #[tokio::test]
    async fn probe_scheduler_respects_shutdown() {
        let probes: Vec<Arc<dyn crate::domain::probe::Probe>> = vec![
            Arc::new(crate::domain::probe::NullProbe),
        ];
        let storage: Arc<dyn StoragePort> = Arc::new(NullStorage);
        let environment = Arc::new(std::sync::RwLock::new(None));
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let handle = spawn_probe_scheduler(probes, storage, environment, task_health, shutdown.clone());
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("probe_scheduler should stop within 2s")
            .expect("task should not panic");
    }
```

- [ ] **Step 3: Run `make check`**

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/infra/tasks.rs
git commit -m "feat(probe): spawn_probe_scheduler — background task with 7-day TTL cleanup"
```

---

### Task 9: AppState + /health extension

**Files:**
- Modify: `cynic-kernel/src/api/rest/types.rs`
- Modify: `cynic-kernel/src/api/rest/health.rs`

- [ ] **Step 1: Add `environment` field to AppState**

Add before the closing `}` of `AppState`:

```rust
    pub environment: Arc<std::sync::RwLock<Option<crate::domain::probe::EnvironmentSnapshot>>>,
```

- [ ] **Step 2: Update all AppState constructions**

Search for all places `AppState` is constructed (main.rs and test files) and add `environment: Arc::new(std::sync::RwLock::new(None))`.

- [ ] **Step 3: Add `environment` to /health response**

In `health.rs`, inside the **authenticated** `serde_json::json!({...})` block (after line ~151), add. Do NOT add to the unauthenticated early-return branch (lines 85-94):

```rust
    "environment": state.environment.read().map(|e| e.as_ref()).unwrap_or(&None),
```

This serializes the latest snapshot or `null` if no probe has run yet.

- [ ] **Step 4: Run `make check`**

Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/api/rest/types.rs cynic-kernel/src/api/rest/health.rs
git commit -m "feat(probe): /health exposes environment snapshot from probe system"
```

---

### Task 10: Wire everything in main.rs

**Files:**
- Modify: `cynic-kernel/src/main.rs`

- [ ] **Step 1: Create the environment Arc and probes**

In main.rs, after the AppState construction section but before background task spawns:

```rust
    // ── Probe system (proprioception) ──
    let environment: Arc<std::sync::RwLock<Option<domain::probe::EnvironmentSnapshot>>> =
        Arc::new(std::sync::RwLock::new(None));
    let backup_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".surrealdb")
        .join("backups");
    let probes: Vec<Arc<dyn domain::probe::Probe>> = vec![
        Arc::new(infra::probes::ResourceProbe),
        Arc::new(infra::probes::BackupProbe::new(backup_dir)),
    ];
```

- [ ] **Step 2: Wire environment into AppState**

Add `environment: Arc::clone(&environment),` to the AppState struct literal.

- [ ] **Step 3: Spawn the probe scheduler**

Add after the last `spawn_*` call:

```rust
    infra::tasks::spawn_probe_scheduler(
        probes,
        Arc::clone(&storage_port),
        Arc::clone(&environment),
        Arc::clone(&task_health),
        shutdown.clone(),
    );
    klog!("[Ring 2] Probe scheduler started (resource: 5min, backup: 1h)");
```

- [ ] **Step 4: Run `make check`**

Expected: All tests pass. This is the full integration point.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/main.rs
git commit -m "feat(probe): wire probe system — ResourceProbe + BackupProbe live in Ring 2"
```

---

### Task 11: Integration test — storage round-trip

**Files:**
- Modify: `cynic-kernel/tests/integration_storage.rs` (or new test file)

- [ ] **Step 1: Add integration test (ignored — requires SurrealDB)**

```rust
#[tokio::test]
#[ignore] // requires SurrealDB at localhost:8000
async fn infra_snapshot_roundtrip() {
    // 1. Create SurrealHttpStorage, init
    // 2. Create a test EnvironmentSnapshot with 2 probe results
    // 3. store_infra_snapshot(&snap)
    // 4. list_infra_snapshots(1) → assert 1 result, check overall matches
    // 5. cleanup_infra_snapshots(0) → assert 1 deleted
    // 6. list_infra_snapshots(1) → assert 0 results
}
```

- [ ] **Step 2: Run `make check`** (test is ignored without SurrealDB flag)

- [ ] **Step 3: Run integration test if SurrealDB available**

Run: `cargo test -p cynic-kernel --test integration_storage infra_snapshot_roundtrip -- --ignored --release`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/tests/integration_storage.rs
git commit -m "test(probe): infra_snapshot storage round-trip integration test"
```

---

### Task 12: Final verification + tag

- [ ] **Step 1: Run full `make check`**

Run: `make check`
Expected: All tests pass, all lints pass, no warnings.

- [ ] **Step 2: Run `make e2e`** (if kernel is running)

Verify `/health` now includes `"environment"` field in authenticated response.

- [ ] **Step 3: Count new tests**

Run: `cargo test -p cynic-kernel --lib -- --list 2>&1 | grep probe | wc -l`
Expected: ≥ 8 new tests (domain, resource, backup, scheduler, task_health, tasks)

- [ ] **Step 4: Final commit if any loose changes**

```bash
git status --short
# If clean: done. If dirty: commit remaining files.
```
