# Organ Consolidation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the InferenceOrgan alive — delete dead tissue, connect quality gate to Judge, add recovery, expose DogStats to /metrics and /health, track real latency.

**Architecture:** The organ senses quality (DogStats + ParseFailureGate) and now ACTS on it (gates Dogs in Judge), RECOVERS (Degraded→Healthy), and EXPOSES data (/metrics + /health). No transport layer — ChatPort remains the transport contract.

**Tech Stack:** Rust 1.94+, tokio, axum, std::collections::VecDeque

**Spec:** `docs/superpowers/specs/2026-04-03-organ-consolidation-design.md`

**Precondition:** `make check` passes on current branch before starting.

---

## File Structure

```
MODIFIED files:
  cynic-kernel/src/organ/mod.rs         — trim dead methods, add is_quality_degraded, add recovery, add dog_quality_snapshot exposure
  cynic-kernel/src/organ/registry.rs    — delete dead types, simplify Backend struct
  cynic-kernel/src/organ/health.rs      — delete alias, VecDeque fix, add latency tracking
  cynic-kernel/src/judge.rs             — add quality gate check, pass elapsed_ms
  cynic-kernel/src/main.rs:217-243      — simplify organ construction
  cynic-kernel/src/domain/metrics.rs    — add organ quality gauges
  cynic-kernel/src/api/rest/health.rs   — wire organ metrics into /metrics, organ quality into /health

DELETED files:
  cynic-kernel/src/organ/router.rs      — 100% dead code
```

---

### Task 1: Delete router.rs + clean mod.rs declaration

**Files:**
- Delete: `cynic-kernel/src/organ/router.rs`
- Modify: `cynic-kernel/src/organ/mod.rs:9-11`

- [ ] **Step 1: Delete router.rs**

```bash
rm cynic-kernel/src/organ/router.rs
```

- [ ] **Step 2: Remove `pub mod router;` from mod.rs**

In `organ/mod.rs`, remove line 11:
```rust
// BEFORE (line 9-11):
pub mod health;
pub mod registry;
pub mod router;

// AFTER:
pub mod health;
pub mod registry;
```

- [ ] **Step 3: Remove `use crate::domain::chat::InferenceProfile;` if present**

Check `organ/mod.rs` for any remaining import from router. The `use` statements at line 13-15 should not reference router types.

- [ ] **Step 4: Verify build**

Run: `cargo build 2>&1 | tail -5`
Expected: compiles (router was dead — nothing referenced it).

- [ ] **Step 5: Run tests**

Run: `cargo test --lib organ 2>&1 | tail -5`
Expected: 18 tests pass (was 25 — 7 router tests removed).

- [ ] **Step 6: Commit**

```bash
git add -u cynic-kernel/src/organ/
git commit -m "refactor(organ): delete dead router.rs (Phase A)

100% dead code — zero production callers. API (takes &[Backend])
incompatible with Arc<Mutex<BackendEntry>> internals. Phase 2 routing
will be redesigned when wired. Design decisions preserved in spec."
```

---

### Task 2: Trim registry.rs — delete dead types

**Files:**
- Modify: `cynic-kernel/src/organ/registry.rs`

- [ ] **Step 1: Delete dead types from registry.rs**

Remove these types entirely from `registry.rs`:
- `NodeId` (line 8)
- `ClusterId` (line 13-14)
- `CapabilityThreshold` struct + impl (lines 52-67)
- `RemediationConfig` struct (lines 83-89)
- `ClusterStrategy` enum (lines 110-115)
- `Cluster` struct (lines 118-124)
- `Node` struct (lines 129-134)

- [ ] **Step 2: Delete dead fields from MeasuredCapabilities**

In `MeasuredCapabilities` (line 31-48), remove:
- `scoring_in_range_rate: f64`
- `mean_latency_ms: u32`
- `tokens_per_second: f32`

After trimming, `MeasuredCapabilities` should be:
```rust
#[derive(Debug, Clone)]
pub struct MeasuredCapabilities {
    pub json_valid_rate: f64,
}

impl Default for MeasuredCapabilities {
    fn default() -> Self {
        Self {
            json_valid_rate: 0.0,  // K14: unknown = pessimistic
        }
    }
}
```

- [ ] **Step 3: Simplify Backend struct**

Remove fields: `node_id`, `endpoint`, `model`, `timeout_secs`, `remediation`.
After:
```rust
#[derive(Debug, Clone)]
pub struct Backend {
    pub id: BackendId,
    pub declared: DeclaredCapabilities,
    pub measured: MeasuredCapabilities,
    pub health: BackendHealth,
}
```

- [ ] **Step 4: Verify build**

Run: `cargo build 2>&1 | tail -10`
Expected: FAIL — `organ/mod.rs` and `main.rs` still reference deleted fields. That's expected, we fix them in Tasks 3 and 4.

- [ ] **Step 5: Do NOT commit yet** — wait for Tasks 3 and 4 to make it compile.

---

### Task 3: Trim organ/mod.rs — fix compilation after registry trim

**Files:**
- Modify: `cynic-kernel/src/organ/mod.rs`

- [ ] **Step 1: Update imports**

In `mod.rs` line 14, remove `BackendHealth` from imports if it was used only for router. Keep `Backend, BackendId, MeasuredCapabilities` — still needed. Remove any import of deleted types (`NodeId`, `RemediationConfig`, etc.).

- [ ] **Step 2: Simplify `BackendEntry` and `make_backend` test helper**

`BackendEntry` (line 21-25) stays the same — it wraps `Backend`, which is now smaller.

Update the test helper `make_backend` (lines 155-167) to match trimmed Backend:
```rust
fn make_backend(id: &str) -> Backend {
    Backend {
        id: BackendId(id.to_string()),
        declared: DeclaredCapabilities::default(),
        measured: MeasuredCapabilities::default(),
        health: BackendHealth::Healthy,
    }
}
```

- [ ] **Step 3: Remove `scoring_in_range_rate` writes from `update_stats_entry`**

In `update_stats_entry()`, the Success branch (around line 93-96) currently reads:
```rust
// BEFORE (Success branch):
let rate = guard.stats.json_valid_rate();
guard.backend.measured.json_valid_rate = rate;        // KEEP this line
guard.backend.measured.scoring_in_range_rate = rate;  // DELETE this line
```

The Failure branch (around line 109-112) currently reads:
```rust
// BEFORE (Failure branch, inside gate-tripped block):
let rate = guard.stats.json_valid_rate();
guard.backend.measured.json_valid_rate = rate;        // KEEP this line
guard.backend.measured.scoring_in_range_rate = rate;  // DELETE this line
```

Remove ONLY the `scoring_in_range_rate` writes. Keep the `json_valid_rate` writes — those are consumed.

- [ ] **Step 4: Delete dead methods from InferenceOrgan**

Remove these methods — zero production callers:
- `InferenceOrgan::backend_ids()` (lines 72-74)
- `InferenceOrgan::overall_valid_rate()` (lines 118-128)
- `InferenceOrgan::measured_snapshot()` (lines 131-140)

Keep `stats_snapshot()` — it will be superseded by `BackendHandle::stats_snapshot()` in Task 7, but is still used by existing tests until then.

- [ ] **Step 5: Do NOT build yet** — main.rs still broken. Continue to Task 4.

NOTE: After Task 4, `InferenceOrgan` is intentionally dropped after boot in `main.rs`. This is correct — it is a factory. The `BackendHandle` values (which contain `Arc<Mutex<BackendEntry>>`) survive via the Judge.

---

### Task 4: Update main.rs organ construction + build + commit Phase A

**Files:**
- Modify: `cynic-kernel/src/main.rs:217-243`
- Modify: `cynic-kernel/src/organ/health.rs` (VecDeque fix)

- [ ] **Step 1: Simplify organ construction in main.rs**

Replace the organ_backend construction at lines 218-241 with:
```rust
let organ_backend = organ::registry::Backend {
    id: organ::registry::BackendId(cfg.name.clone()),
    declared: organ::registry::DeclaredCapabilities {
        json: cfg.json_mode,
        thinking: !cfg.disable_thinking,
        scoring: true,
        ..Default::default()
    },
    measured: organ::registry::MeasuredCapabilities::default(),
    health: organ::registry::BackendHealth::Healthy,
};
```

- [ ] **Step 2: Delete `scoring_in_range_rate()` alias from health.rs**

Remove the method at `health.rs:84-87`:
```rust
// DELETE this method:
pub fn scoring_in_range_rate(&self) -> f64 {
    self.json_valid_rate()
}
```

- [ ] **Step 3: Fix ParseFailureGate — VecDeque**

In `health.rs`, replace `Vec<bool>` with `VecDeque<bool>`:

Add import at top of file:
```rust
use std::collections::VecDeque;
```

Change `ParseFailureGate` struct (line 116-120):
```rust
pub struct ParseFailureGate {
    window: VecDeque<bool>,
    capacity: usize,
}
```

Change `new()` (line 123-127):
```rust
pub fn new() -> Self {
    Self {
        window: VecDeque::new(),
        capacity: 10,
    }
}
```

Change `push()` (line 138-143):
```rust
fn push(&mut self, ok: bool) {
    if self.window.len() >= self.capacity {
        self.window.pop_front();
    }
    self.window.push_back(ok);
}
```

- [ ] **Step 4: Build**

Run: `cargo build 2>&1 | tail -10`
Expected: compiles. All dead types/fields removed, all references updated.

- [ ] **Step 5: Run tests**

Run: `cargo test --lib organ 2>&1 | tail -5`
Expected: 18 tests pass (was 25, lost 7 router tests).

Run: `cargo test 2>&1 | tail -5`
Expected: all tests pass (360 - 7 router = 353+).

- [ ] **Step 6: Run make check**

Run: `make check 2>&1 | tail -20`
Expected: GREEN. Clippy, lint-rules, lint-drift all pass.

- [ ] **Step 7: Commit Phase A**

```bash
git add -u cynic-kernel/
git commit -m "refactor(organ): Phase A — delete dead tissue, simplify registry

Remove 13 dead symbols: Node, Cluster, ClusterId, ClusterStrategy,
CapabilityThreshold, RemediationConfig, router.rs (entire file),
scoring_in_range_rate, mean_latency_ms, tokens_per_second.
Simplify Backend to id+declared+measured+health.
Fix ParseFailureGate to use VecDeque (O(1) ring buffer)."
```

---

### Task 5: Phase B — Add `is_quality_degraded()` + quality gate in Judge

**Files:**
- Modify: `cynic-kernel/src/organ/mod.rs`
- Modify: `cynic-kernel/src/judge.rs:202-204`

- [ ] **Step 1: Write failing test for is_quality_degraded**

Add to `organ/mod.rs` tests:
```rust
#[test]
fn healthy_backend_is_not_quality_degraded() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("dog-a"));
    assert!(!handle.is_quality_degraded());
}

#[test]
fn degraded_backend_is_quality_degraded() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("dog-a"));
    // Trip the gate: 6 failures out of 10
    for _ in 0..4 {
        InferenceOrgan::update_stats_entry(&handle, ScoreOutcome::Success);
    }
    for _ in 0..6 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ParseError),
        );
    }
    assert!(handle.is_quality_degraded());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib organ::tests::healthy_backend 2>&1 | tail -5`
Expected: FAIL — `is_quality_degraded` not defined.

- [ ] **Step 3: Implement `is_quality_degraded()` on BackendHandle**

Add to `organ/mod.rs`, impl block for `BackendHandle` (after line 30):
```rust
impl BackendHandle {
    /// Returns true if this backend's quality gate has tripped.
    /// Acquires Mutex briefly, reads health, releases. No async, no hold across .await.
    /// K14: Mutex poison = assume degraded (safe default).
    pub fn is_quality_degraded(&self) -> bool {
        self.0
            .lock()
            .ok()
            .map_or(true, |guard| {
                matches!(
                    guard.backend.health,
                    BackendHealth::Degraded { .. } | BackendHealth::Dead { .. }
                )
            })
    }
}
```

Add `BackendHealth` to the imports at line 14 if not already imported:
```rust
use crate::organ::registry::{Backend, BackendHealth, BackendId, MeasuredCapabilities};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib organ::tests 2>&1 | tail -5`
Expected: 20 tests pass (18 existing + 2 new).

- [ ] **Step 5: Wire quality gate into Judge**

In `judge.rs`, the pre-evaluation filter is at line 202-204:
```rust
// BEFORE:
let futures: Vec<_> = dog_breaker_pairs
    .iter()
    .filter(|(_, cb)| cb.should_allow())
```

Extract a helper closure used in BOTH filter sites (line 202 and line 221):
```rust
// Define before both filter sites:
let is_dog_allowed = |dog: &dyn Dog, cb: &Arc<dyn HealthGate>| -> bool {
    if !cb.should_allow() {
        return false;
    }
    let dog_idx = self.dogs.iter().position(|d| d.id() == dog.id());
    if let Some(handle) = dog_idx.and_then(|idx| self.organ_handles[idx].as_ref()) {
        if handle.is_quality_degraded() {
            tracing::warn!(dog_id = %dog.id(), "Dog skipped — organ quality degraded");
            return false;
        }
    }
    true
};
```

Then update BOTH filter sites:
```rust
// Line 202 (futures construction):
.filter(|(dog, cb)| is_dog_allowed(*dog, cb))

// Line 221 (wall-clock timeout calculation):
.filter(|(dog, cb)| is_dog_allowed(*dog, cb))
```

NOTE: The borrow checker may require adjusting lifetimes. If the closure can't borrow `self`, inline the organ check at both sites instead. The key requirement: both filters must apply the same gate — futures filter AND wall-clock timeout filter.

- [ ] **Step 6: Write test for quality gate in Judge**

Add to `judge.rs` tests, using the existing `FixedDog` and `test_judge` helpers (defined at lines 600-638):
```rust
#[tokio::test]
async fn quality_degraded_dog_is_skipped() {
    let good_dog = FixedDog {
        name: "good".into(),
        scores: AxiomScores {
            fidelity: 0.5, phi: 0.5, verify: 0.5,
            culture: 0.5, burn: 0.5, sovereignty: 0.5,
            reasoning: AxiomReasoning::default(),
            ..Default::default()
        },
    };
    let bad_dog = FixedDog {
        name: "bad".into(),
        scores: AxiomScores {
            fidelity: 0.1, phi: 0.1, verify: 0.1,
            culture: 0.1, burn: 0.1, sovereignty: 0.1,
            reasoning: AxiomReasoning::default(),
            ..Default::default()
        },
    };

    // Register "bad" backend as Degraded in the organ
    let mut organ = crate::organ::InferenceOrgan::boot_empty();
    let handle = organ.register_backend(crate::organ::registry::Backend {
        id: crate::organ::registry::BackendId("bad".into()),
        declared: crate::organ::registry::DeclaredCapabilities::default(),
        measured: crate::organ::registry::MeasuredCapabilities::default(),
        health: crate::organ::registry::BackendHealth::Degraded {
            reason: "test quality gate".into(),
            since: std::time::Instant::now(),
        },
    });

    let judge = test_judge(vec![
        Box::new(good_dog),
        Box::new(bad_dog),
    ]).with_organ_handles(vec![None, Some(handle)]);

    let metrics = Arc::new(test_metrics());
    let verdict = judge.evaluate(&test_stimulus(), None, &metrics).await.unwrap();

    // bad dog is quality-degraded → should be skipped
    assert_eq!(verdict.dog_scores.len(), 1);
    assert_eq!(verdict.dog_scores[0].dog_id, "good");
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test --lib judge::tests 2>&1 | tail -10`
Expected: all judge tests pass including the new one.

Run: `cargo test 2>&1 | tail -5`
Expected: all tests pass.

- [ ] **Step 8: Commit Phase B**

```bash
git add cynic-kernel/src/organ/mod.rs cynic-kernel/src/judge.rs
git commit -m "feat(organ): Phase B — quality gate skips degraded Dogs

BackendHandle::is_quality_degraded() reads organ health state.
Judge skips Dogs whose parse failure gate has tripped, same as
circuit breaker skip. CB = infra gate, organ = quality gate."
```

---

### Task 6: Phase C — Recovery (Degraded → Healthy)

**Files:**
- Modify: `cynic-kernel/src/organ/mod.rs`

- [ ] **Step 1: Write failing test for recovery**

Add to `organ/mod.rs` tests:
```rust
#[test]
fn recovery_promotes_degraded_to_healthy() {
    let mut organ = InferenceOrgan::boot_empty();
    let handle = organ.register_backend(make_backend("dog-a"));
    // Trip the gate: fill window with failures
    for _ in 0..10 {
        InferenceOrgan::update_stats_entry(
            &handle,
            ScoreOutcome::Failure(ScoreFailureKind::ParseError),
        );
    }
    assert!(handle.is_quality_degraded());

    // Recover: 10 successes evict all failures from window
    for _ in 0..10 {
        InferenceOrgan::update_stats_entry(&handle, ScoreOutcome::Success);
    }
    // Gate is no longer tripped → should recover to Healthy
    assert!(!handle.is_quality_degraded());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib organ::tests::recovery 2>&1 | tail -5`
Expected: FAIL — backend stays Degraded after recovery (the bug).

- [ ] **Step 3: Add recovery logic in `update_stats_entry`**

In `organ/mod.rs`, `update_stats_entry()`, after the existing gate-trip check (around line 100-113), add recovery:

```rust
// After the existing match block, add:
// Recovery: if gate is no longer tripped and backend was degraded, promote to Healthy.
if !guard.gate.is_tripped() {
    if matches!(guard.backend.health, BackendHealth::Degraded { .. }) {
        tracing::info!(
            backend = %guard.backend.id.0,
            "organ: quality recovered — promoting to Healthy"
        );
        guard.backend.health = BackendHealth::Healthy;
    }
}
```

Place this AFTER the closing brace of the `match kind { ... }` block, before the method returns. It runs for BOTH success and failure outcomes:

```rust
pub fn update_stats_entry(handle: &BackendHandle, kind: ScoreOutcome) {
    let Ok(mut guard) = handle.0.lock() else {
        return;
    };
    match kind {
        ScoreOutcome::Success => {
            // ... existing success logic ...
        }
        ScoreOutcome::Failure(failure_kind) => {
            // ... existing failure logic including gate trip check ...
        }
    }
    // ↓ Recovery goes HERE — after match, before method end ↓
    if !guard.gate.is_tripped() {
        if matches!(guard.backend.health, BackendHealth::Degraded { .. }) {
            tracing::info!(/* ... */);
            guard.backend.health = BackendHealth::Healthy;
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib organ::tests 2>&1 | tail -5`
Expected: all organ tests pass including recovery test.

- [ ] **Step 5: Commit Phase C**

```bash
git add cynic-kernel/src/organ/mod.rs
git commit -m "fix(organ): Phase C — recovery from Degraded to Healthy

When ParseFailureGate clears (10 successes), promote backend
back to Healthy. Fixes bug where degraded backends stayed
degraded forever until process restart."
```

---

### Task 7: Phase D — Expose DogStats via Judge + /metrics

**Files:**
- Modify: `cynic-kernel/src/judge.rs`
- Modify: `cynic-kernel/src/organ/mod.rs`
- Modify: `cynic-kernel/src/organ/health.rs`
- Modify: `cynic-kernel/src/domain/metrics.rs`
- Modify: `cynic-kernel/src/api/rest/health.rs`

- [ ] **Step 1: Add `dog_quality_snapshot()` to Judge**

In `judge.rs`, add a new public method:
```rust
use crate::organ::health::DogStats;

/// Snapshot of organ quality data for each Dog. Reads BackendHandle locks briefly.
/// Returns (dog_id, DogStats) for each dog that has an organ handle.
pub fn dog_quality_snapshot(&self) -> Vec<(String, DogStats)> {
    self.dogs
        .iter()
        .enumerate()
        .filter_map(|(idx, dog)| {
            let handle = self.organ_handles[idx].as_ref()?;
            let stats = handle.0.lock().ok().map(|g| g.stats.clone())?;
            Some((dog.id().to_string(), stats))
        })
        .collect()
}
```

Note: `handle.0` accesses the inner `Arc<Mutex<BackendEntry>>` — this requires `BackendHandle.0` to be accessible from judge.rs. Since `BackendHandle` is defined as `pub struct BackendHandle(Arc<Mutex<BackendEntry>>)` where `BackendEntry` is private, we need an accessor. Add to `BackendHandle`:

```rust
/// Snapshot DogStats from this handle. Returns None if Mutex is poisoned.
pub fn stats_snapshot(&self) -> Option<DogStats> {
    self.0.lock().ok().map(|guard| guard.stats.clone())
}
```

Then `dog_quality_snapshot` becomes:
```rust
pub fn dog_quality_snapshot(&self) -> Vec<(String, DogStats)> {
    self.dogs
        .iter()
        .enumerate()
        .filter_map(|(idx, dog)| {
            let handle = self.organ_handles[idx].as_ref()?;
            let stats = handle.stats_snapshot()?;
            Some((dog.id().to_string(), stats))
        })
        .collect()
}
```

- [ ] **Step 1b: Delete superseded `InferenceOrgan::stats_snapshot()`**

Now that `BackendHandle::stats_snapshot()` exists, delete the organ-level `InferenceOrgan::stats_snapshot()` method (organ/mod.rs, around line 77). It is only called in tests, and those tests should use the new `BackendHandle::stats_snapshot()` instead. Update any tests that call `organ.stats_snapshot(...)` to use `handle.stats_snapshot()`.

- [ ] **Step 2: Add `append_organ_metrics()` to domain/metrics.rs**

Add after `append_dog_metrics()` (line 217):
```rust
/// Append organ quality metrics from DogStats snapshots.
pub fn append_organ_metrics(
    out: &mut String,
    snapshots: &[(String, crate::organ::health::DogStats)],
) {
    use std::fmt::Write;

    // JSON valid rate per Dog (gauge)
    let _ = writeln!(out, "# HELP cynic_dog_json_valid_rate Fraction of valid JSON responses per Dog");
    let _ = writeln!(out, "# TYPE cynic_dog_json_valid_rate gauge");
    for (id, stats) in snapshots {
        let _ = writeln!(out, "cynic_dog_json_valid_rate{{dog=\"{id}\"}} {:.6}", stats.json_valid_rate());
    }

    // Capability limit rate per Dog (gauge)
    let _ = writeln!(out, "# HELP cynic_dog_capability_limit_rate Fraction of capability-limit failures (zero flood + collapse)");
    let _ = writeln!(out, "# TYPE cynic_dog_capability_limit_rate gauge");
    for (id, stats) in snapshots {
        let _ = writeln!(out, "cynic_dog_capability_limit_rate{{dog=\"{id}\"}} {:.6}", stats.capability_limit_rate());
    }

    // Total calls per Dog (counter)
    let _ = writeln!(out, "# HELP cynic_dog_organ_total Total organ-tracked evaluations per Dog");
    let _ = writeln!(out, "# TYPE cynic_dog_organ_total counter");
    for (id, stats) in snapshots {
        let _ = writeln!(out, "cynic_dog_organ_total{{dog=\"{id}\"}} {}", stats.total_calls);
    }

    // Quality failures by mode (counter)
    let _ = writeln!(out, "# HELP cynic_dog_quality_failures Dog quality failures by failure mode");
    let _ = writeln!(out, "# TYPE cynic_dog_quality_failures counter");
    for (id, stats) in snapshots {
        let _ = writeln!(out, "cynic_dog_quality_failures{{dog=\"{id}\",mode=\"zero_flood\"}} {}", stats.zero_flood_count);
        let _ = writeln!(out, "cynic_dog_quality_failures{{dog=\"{id}\",mode=\"collapse\"}} {}", stats.collapse_count);
        let _ = writeln!(out, "cynic_dog_quality_failures{{dog=\"{id}\",mode=\"parse_error\"}} {}", stats.parse_error_count);
        let _ = writeln!(out, "cynic_dog_quality_failures{{dog=\"{id}\",mode=\"timeout\"}} {}", stats.timeout_count);
    }
}
```

- [ ] **Step 3: Wire into /metrics handler**

In `api/rest/health.rs` `metrics_handler()`, after the existing `append_dog_metrics` call (line 267), add:
```rust
// Organ quality metrics
{
    let snapshots = state.judge.dog_quality_snapshot();
    crate::domain::metrics::append_organ_metrics(&mut out, &snapshots);
}
```

- [ ] **Step 4: Wire organ quality into /health handler**

In `api/rest/health.rs` `health_handler()`, after the `dog_health` section (around line 75), add quality info to the authenticated response. In the `dogs` Vec construction (lines 115-131), add a `quality_rate` field:

This requires updating `DogHealthResponse` in `api/rest/types.rs` to include quality data. Check if adding a field is straightforward. Alternatively, add organ quality as a separate section in the JSON response:

After line 131 (where `dogs` Vec is built), add:
```rust
let organ_quality: Vec<serde_json::Value> = state.judge.dog_quality_snapshot()
    .into_iter()
    .map(|(id, stats)| serde_json::json!({
        "dog": id,
        "json_valid_rate": stats.json_valid_rate(),
        "capability_limit_rate": stats.capability_limit_rate(),
        "total_calls": stats.total_calls,
    }))
    .collect();
```

Then include `"organ_quality": organ_quality` in the authenticated JSON response object.

- [ ] **Step 5: Build + test**

Run: `cargo build 2>&1 | tail -5`
Run: `cargo test 2>&1 | tail -5`
Expected: compiles, all tests pass.

- [ ] **Step 6: Commit Phase D**

```bash
git add cynic-kernel/src/judge.rs cynic-kernel/src/organ/ cynic-kernel/src/domain/metrics.rs cynic-kernel/src/api/rest/health.rs
git commit -m "feat(organ): Phase D — expose DogStats via /metrics and /health

Judge::dog_quality_snapshot() reads BackendHandle stats.
New Prometheus gauges: cynic_dog_json_valid_rate,
cynic_dog_capability_limit_rate, cynic_dog_quality_failures.
/health includes organ_quality section in authenticated response."
```

---

### Task 8: Phase E — Latency tracking (real data)

**Files:**
- Modify: `cynic-kernel/src/organ/mod.rs`
- Modify: `cynic-kernel/src/organ/health.rs`
- Modify: `cynic-kernel/src/judge.rs`
- Modify: `cynic-kernel/src/domain/metrics.rs`

- [ ] **Step 1: Write failing test for latency tracking**

Add to `organ/health.rs` tests:
```rust
#[test]
fn dog_stats_tracks_mean_latency() {
    let mut s = DogStats::new();
    assert_eq!(s.mean_latency_ms(), 0.0);  // 0.0 when no successes
    s.record_success_with_latency(100);
    s.record_success_with_latency(200);
    s.record_success_with_latency(300);
    assert!((s.mean_latency_ms() - 200.0).abs() < 1e-10);
}
```

- [ ] **Step 2: Run test to verify it fails**

Expected: FAIL — `record_success_with_latency` and `mean_latency_ms` don't exist.

- [ ] **Step 3: Add latency tracking to DogStats**

In `health.rs`, add to `DogStats` struct:
```rust
/// Cumulative latency of successful calls (ms). Used to compute mean.
pub total_latency_ms: u64,
```

Add to `DogStats::new()`:
```rust
total_latency_ms: 0,
```

Add methods:
```rust
pub fn record_success_with_latency(&mut self, elapsed_ms: u64) {
    self.record_success();
    self.total_latency_ms += elapsed_ms;
}

/// Mean latency of successful calls in milliseconds.
/// Returns 0.0 when no successful calls recorded.
pub fn mean_latency_ms(&self) -> f64 {
    if self.success_count == 0 {
        return 0.0;
    }
    self.total_latency_ms as f64 / self.success_count as f64
}
```

- [ ] **Step 4: Run test**

Run: `cargo test --lib organ::health::tests::dog_stats_tracks 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 5: Change ScoreOutcome::Success to carry elapsed_ms**

In `organ/mod.rs`, change the enum:
```rust
pub enum ScoreOutcome {
    Success { elapsed_ms: u64 },
    Failure(ScoreFailureKind),
}
```

Update `update_stats_entry` to use the new field:
```rust
ScoreOutcome::Success { elapsed_ms } => {
    guard.stats.record_success_with_latency(elapsed_ms);
    guard.gate.record_success();
    let rate = guard.stats.json_valid_rate();
    guard.backend.measured.json_valid_rate = rate;
}
```

- [ ] **Step 6: Fix ALL match sites that use `ScoreOutcome::Success`**

This enum variant change affects every file that constructs `ScoreOutcome::Success`:

1. `organ/mod.rs` tests: Replace all `ScoreOutcome::Success` with `ScoreOutcome::Success { elapsed_ms: 0 }`. Affected tests: `update_success_increments_rate`, `gate_trips_and_degrades_backend`, `overall_valid_rate_averages_backends`, `healthy_backend_is_not_quality_degraded`, `degraded_backend_is_quality_degraded`, `recovery_promotes_degraded_to_healthy`.

2. `judge.rs` tests: The `quality_degraded_dog_is_skipped` test from Task 5 does NOT directly construct `ScoreOutcome::Success` — it uses `FixedDog` which returns scores through the Dog trait. No change needed there. But check for any other test that constructs `ScoreOutcome::Success` directly.

Run `grep -rn 'ScoreOutcome::Success' cynic-kernel/src/` to find all sites.

- [ ] **Step 7: Update judge.rs call sites**

In `judge.rs` line 254, change:
```rust
// BEFORE:
InferenceOrgan::update_stats_entry(h, ScoreOutcome::Success);

// AFTER:
InferenceOrgan::update_stats_entry(h, ScoreOutcome::Success { elapsed_ms });
```

The `elapsed_ms` variable is already in scope at this point (from the evaluation result tuple at line 240).

- [ ] **Step 8: Add latency to Prometheus metrics + remove old latency metric (R12)**

In `domain/metrics.rs` `append_organ_metrics()`, add:
```rust
// Mean latency per Dog (gauge) — replaces cynic_dog_latency_ms from append_dog_metrics
let _ = writeln!(out, "# HELP cynic_dog_mean_latency_ms Mean successful evaluation latency in milliseconds (organ-measured)");
let _ = writeln!(out, "# TYPE cynic_dog_mean_latency_ms gauge");
for (id, stats) in snapshots {
    let _ = writeln!(out, "cynic_dog_mean_latency_ms{{dog=\"{id}\"}} {:.1}", stats.mean_latency_ms());
}
```

Also in `domain/metrics.rs`, REMOVE the old `cynic_dog_latency_ms` gauge block from `append_dog_metrics()` (lines 161-173). The organ's `cynic_dog_mean_latency_ms` is the authoritative source (R12: one value, one source). Keep `cynic_dog_requests_total`, `cynic_dog_failures`, `cynic_dog_tokens_total`, and `cynic_dog_circuit_breaker` — those are not duplicated by organ metrics.

- [ ] **Step 9: Build + test**

Run: `cargo build 2>&1 | tail -5`
Run: `cargo test 2>&1 | tail -5`
Expected: compiles, all tests pass.

- [ ] **Step 10: Run make check**

Run: `make check 2>&1 | tail -20`
Expected: GREEN.

- [ ] **Step 11: Commit Phase E**

```bash
git add cynic-kernel/src/organ/ cynic-kernel/src/judge.rs cynic-kernel/src/domain/metrics.rs
git commit -m "feat(organ): Phase E — track real latency from Judge evaluations

ScoreOutcome::Success now carries elapsed_ms. DogStats computes
incremental mean latency. Exposed via cynic_dog_mean_latency_ms
Prometheus gauge. No more stuck-at-sentinel values."
```

---

### Task 9: Final verification

**Files:** None (verification only)

- [ ] **Step 1: Run full make check**

Run: `make check 2>&1 | tail -30`
Expected: GREEN — build + test + clippy + lint-rules + lint-drift + audit.

- [ ] **Step 2: Verify zero dead symbols in organ/**

Run: `grep -rn 'pub fn\|pub struct\|pub enum\|pub type' cynic-kernel/src/organ/ --include='*.rs' | grep -v '#\[cfg(test)\]' | grep -v 'mod tests'`

For each public symbol, verify it has a production caller (not just test callers).

- [ ] **Step 3: Verify success criteria**

1. `make check` passes ✓ (Step 1)
2. Zero dead symbols in organ/ ✓ (Step 2)
3. Quality gate test passes: `cargo test --lib organ::tests::degraded_backend_is_quality_degraded`
4. Recovery test passes: `cargo test --lib organ::tests::recovery_promotes`
5. Metrics wired: `grep cynic_dog_json_valid_rate cynic-kernel/src/domain/metrics.rs`
6. Health wired: `grep organ_quality cynic-kernel/src/api/rest/health.rs`
7. Latency tracked: `cargo test --lib organ::health::tests::dog_stats_tracks`

- [ ] **Step 4: Commit verification**

```bash
git log --oneline -8  # Review commit chain
```

Expected: Phase A → Phase B → Phase C → Phase D → Phase E, clean progression.
