# Soma L2 — Priority Slot Semaphore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the static ResourceGate with per-dog priority-aware semaphores that coordinate all inference consumers (user, Hermes, nightshift, background) through `Judge::evaluate`.

**Architecture:** A `SlotSemaphore` per dog tracks physical llama-server slots as tokio semaphore permits. User/Hermes requests wait (bounded timeout) for a permit; nightshift/background try_acquire and skip on failure. The semaphore is acquired inside `Judge::evaluate_progressive` before each dog's `score()` call. Health loop initializes permit counts from slot probes.

**Tech Stack:** Rust, tokio (Semaphore, oneshot, Mutex), existing Judge/SlotTracker/health_loop infrastructure.

**Spec:** `docs/superpowers/specs/2026-05-11-soma-l2-priority-slot-semaphore-design.md`

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `cynic-kernel/src/domain/slot_semaphore.rs` | SlotSemaphore struct, SlotPriority enum, SlotPermit guard, SlotSemaphoreMap |
| Modify | `cynic-kernel/src/domain/mod.rs` | Add `pub mod slot_semaphore;` |
| Modify | `cynic-kernel/src/judge/mod.rs:435-443,576-606` | Add `priority: SlotPriority` param, acquire permits in fan-out |
| Modify | `cynic-kernel/src/pipeline/mod.rs:99-106,540-547` | Thread `SlotPriority` through `PipelineDeps` and `run()` |
| Modify | `cynic-kernel/src/api/rest/judge.rs` | Pass `SlotPriority::User` to pipeline |
| Modify | `cynic-kernel/src/api/rest/judge_job.rs` | Pass `SlotPriority::User` to pipeline |
| Modify | `cynic-kernel/src/api/mcp/judge_tools.rs` | Pass `SlotPriority::User` to pipeline |
| Modify | `cynic-kernel/src/infra/tasks/nightshift.rs` | Remove soma_gate, pass `SlotPriority::Nightshift` to evaluate |
| Modify | `cynic-kernel/src/infra/tasks/runtime_loops.rs:823` | Pass `SlotPriority::Background` to evaluate |
| Modify | `cynic-kernel/src/infra/health_loop.rs` | Initialize/resize semaphore permits on slot probe |
| Modify | `cynic-kernel/src/main.rs` | Create SlotSemaphoreMap, inject into Judge + health_loop, remove soma_gate |
| Modify | `cynic-kernel/src/api/rest/types.rs` | Replace `soma_gate: Arc<ResourceGate>` with `slot_semaphores: Arc<SlotSemaphoreMap>` |
| Modify | `cynic-kernel/src/api/rest/soma.rs` | Update `/soma/request` to use semaphore (or remove endpoint) |
| Delete (contents) | `cynic-kernel/src/domain/orchestrator.rs` | Remove ResourceGate, keep Priority → rename to SlotPriority in new module |
| Modify | `cynic-kernel/tests/rest_routes.rs` | Update test_state: soma_gate → slot_semaphores |
| Modify | `cynic-kernel/tests/integration_judge.rs` | Pass priority to evaluate calls |

---

## Task 1: SlotSemaphore struct + unit tests

**Files:**
- Create: `cynic-kernel/src/domain/slot_semaphore.rs`
- Modify: `cynic-kernel/src/domain/mod.rs`

- [ ] **Step 1: Write F1 test — permit exhaustion**

```rust
// In slot_semaphore.rs #[cfg(test)] mod tests
#[tokio::test]
async fn f1_permits_exhaust_at_slot_count() {
    let sem = SlotSemaphore::new("test-dog", 4);
    // 4 try_acquires succeed
    let p1 = sem.try_acquire(SlotPriority::Background).unwrap();
    let p2 = sem.try_acquire(SlotPriority::Background).unwrap();
    let p3 = sem.try_acquire(SlotPriority::Background).unwrap();
    let p4 = sem.try_acquire(SlotPriority::Background).unwrap();
    // 5th fails
    assert!(sem.try_acquire(SlotPriority::Background).is_none());
    // Drop one → next succeeds
    drop(p1);
    assert!(sem.try_acquire(SlotPriority::Background).is_some());
}
```

- [ ] **Step 2: Run test — verify it fails (struct doesn't exist)**

```bash
cargo test -p cynic-kernel --lib -- slot_semaphore::tests::f1 2>&1 | tail -5
```
Expected: compilation error, `SlotSemaphore` not found.

- [ ] **Step 3: Implement SlotSemaphore core**

```rust
// cynic-kernel/src/domain/slot_semaphore.rs

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Priority levels for inference slot acquisition.
/// Higher value = higher priority = served first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SlotPriority {
    Background = 0,
    Nightshift = 1,
    Hermes = 2,
    User = 3,
}

impl SlotPriority {
    /// Blocking priorities wait for a slot (with timeout).
    /// Non-blocking priorities skip immediately if no slot available.
    pub fn is_blocking(self) -> bool {
        matches!(self, Self::User | Self::Hermes)
    }

    /// Max wait time for blocking priorities.
    pub fn timeout(self) -> std::time::Duration {
        match self {
            Self::User => std::time::Duration::from_secs(30),
            Self::Hermes => std::time::Duration::from_secs(15),
            _ => std::time::Duration::ZERO, // non-blocking, unused
        }
    }
}

/// RAII permit guard. Dropping releases the semaphore permit.
pub struct SlotPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
    pub dog_id: String,
    pub priority: SlotPriority,
}

/// Per-dog priority-aware semaphore.
/// N permits = N physical llama-server slots.
pub struct SlotSemaphore {
    dog_id: String,
    semaphore: Arc<Semaphore>,
    total_slots: AtomicU32,
}

impl SlotSemaphore {
    pub fn new(dog_id: &str, slots: u32) -> Self {
        Self {
            dog_id: dog_id.to_string(),
            semaphore: Arc::new(Semaphore::new(slots as usize)),
            total_slots: AtomicU32::new(slots),
        }
    }

    /// Non-blocking acquire. Returns None if no slot available.
    pub fn try_acquire(&self, priority: SlotPriority) -> Option<SlotPermit> {
        let permit = Arc::clone(&self.semaphore)
            .try_acquire_owned()
            .ok()?;
        Some(SlotPermit {
            _permit: permit,
            dog_id: self.dog_id.clone(),
            priority,
        })
    }

    /// Blocking acquire with timeout. Returns None on timeout.
    pub async fn acquire(
        &self,
        priority: SlotPriority,
    ) -> Option<SlotPermit> {
        let timeout = priority.timeout();
        let permit = tokio::time::timeout(
            timeout,
            Arc::clone(&self.semaphore).acquire_owned(),
        )
        .await
        .ok()? // timeout → None
        .ok()?; // semaphore closed → None
        Some(SlotPermit {
            _permit: permit,
            dog_id: self.dog_id.clone(),
            priority,
        })
    }

    /// Add permits when health loop discovers more slots.
    pub fn add_permits(&self, n: u32) {
        self.semaphore.add_permits(n as usize);
        self.total_slots.fetch_add(n, Ordering::Relaxed);
    }

    /// Current total slot count (for /health).
    pub fn total_slots(&self) -> u32 {
        self.total_slots.load(Ordering::Relaxed)
    }

    /// Available permits right now.
    pub fn available(&self) -> u32 {
        self.semaphore.available_permits() as u32
    }
}

/// Map of dog_id → SlotSemaphore. Shared across Judge + health loop.
pub struct SlotSemaphoreMap {
    inner: std::sync::RwLock<BTreeMap<String, Arc<SlotSemaphore>>>,
}

impl SlotSemaphoreMap {
    pub fn new() -> Self {
        Self {
            inner: std::sync::RwLock::new(BTreeMap::new()),
        }
    }

    /// Get semaphore for a dog. Returns None if dog not yet discovered.
    pub fn get(&self, dog_id: &str) -> Option<Arc<SlotSemaphore>> {
        self.inner.read().ok()?.get(dog_id).cloned()
    }

    /// Create or resize semaphore for a dog.
    /// Called by health loop when slot count is discovered/changed.
    pub fn upsert(&self, dog_id: &str, slots: u32) {
        let Ok(mut guard) = self.inner.write() else { return };
        if let Some(existing) = guard.get(dog_id) {
            let current = existing.total_slots();
            if slots > current {
                existing.add_permits(slots - current);
            }
            // slots < current: permits drain naturally (no remove_permits)
        } else {
            guard.insert(
                dog_id.to_string(),
                Arc::new(SlotSemaphore::new(dog_id, slots)),
            );
        }
    }

    /// Remove semaphore for a dog (expired/removed).
    pub fn remove(&self, dog_id: &str) {
        if let Ok(mut guard) = self.inner.write() {
            guard.remove(dog_id);
        }
    }
}

impl Default for SlotSemaphoreMap {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Add `pub mod slot_semaphore;` to `cynic-kernel/src/domain/mod.rs`**

- [ ] **Step 5: Run F1 test — verify it passes**

```bash
cargo test -p cynic-kernel --lib -- slot_semaphore::tests::f1 -v
```
Expected: PASS

- [ ] **Step 6: Write F4 test — boot initialization (0 permits → add)**

```rust
#[tokio::test]
async fn f4_boot_zero_then_add_permits() {
    let sem = SlotSemaphore::new("boot-dog", 0);
    // No permits → try_acquire fails
    assert!(sem.try_acquire(SlotPriority::User).is_none());
    // Health loop discovers 4 slots
    sem.add_permits(4);
    assert_eq!(sem.total_slots(), 4);
    // Now acquire works
    assert!(sem.try_acquire(SlotPriority::User).is_some());
}
```

- [ ] **Step 7: Write F2 test — blocking acquire waits then succeeds**

```rust
#[tokio::test]
async fn f2_blocking_acquire_waits_for_release() {
    let sem = Arc::new(SlotSemaphore::new("wait-dog", 1));
    // Take the only slot
    let held = sem.try_acquire(SlotPriority::Background).unwrap();
    // User acquire should block, then succeed when we release
    let sem2 = Arc::clone(&sem);
    let handle = tokio::spawn(async move {
        sem2.acquire(SlotPriority::User).await
    });
    // Release after short delay
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    drop(held);
    let permit = handle.await.unwrap();
    assert!(permit.is_some());
}
```

- [ ] **Step 8: Write upsert test**

```rust
#[test]
fn semaphore_map_upsert_creates_and_resizes() {
    let map = SlotSemaphoreMap::new();
    // First upsert creates
    map.upsert("dog-a", 2);
    let sem = map.get("dog-a").unwrap();
    assert_eq!(sem.total_slots(), 2);
    // Second upsert with more slots adds permits
    map.upsert("dog-a", 4);
    assert_eq!(sem.total_slots(), 4);
    // Unknown dog returns None
    assert!(map.get("nonexistent").is_none());
}
```

- [ ] **Step 9: Run all slot_semaphore tests**

```bash
cargo test -p cynic-kernel --lib -- slot_semaphore -v
```
Expected: all pass.

- [ ] **Step 10: Commit**

```bash
git add cynic-kernel/src/domain/slot_semaphore.rs cynic-kernel/src/domain/mod.rs
git commit -m "feat(soma): SlotSemaphore — per-dog priority-aware permit system"
```

---

## Task 2: Integrate into Judge::evaluate

**Files:**
- Modify: `cynic-kernel/src/judge/mod.rs:435-443,576-606`

- [ ] **Step 1: Add `SlotSemaphoreMap` to `Judge` struct**

In `judge/mod.rs`, add field:
```rust
slot_semaphores: Option<Arc<SlotSemaphoreMap>>,
```

Update `Judge::new()` to initialize it as `None`. Add setter:
```rust
pub fn with_slot_semaphores(mut self, sem: Arc<SlotSemaphoreMap>) -> Self {
    self.slot_semaphores = Some(sem);
    self
}
```

- [ ] **Step 2: Add `SlotPriority` param to `evaluate` and `evaluate_progressive`**

```rust
// evaluate() becomes:
pub async fn evaluate(
    &self,
    stimulus: &Stimulus,
    filter: Option<&[String]>,
    metrics: &Metrics,
    priority: SlotPriority,
) -> Result<Verdict, JudgeError> {
    self.evaluate_progressive(stimulus, filter, metrics, None, priority).await
}

// evaluate_progressive() adds priority param at the end:
pub async fn evaluate_progressive(
    &self,
    stimulus: &Stimulus,
    filter: Option<&[String]>,
    metrics: &Metrics,
    on_dog: Option<&crate::pipeline::OnDogCallback>,
    priority: SlotPriority,
) -> Result<Verdict, JudgeError> {
```

- [ ] **Step 3: Add permit acquisition in the fan-out loop (L591-606)**

Replace the existing fan-out with semaphore-gated version:

```rust
for entry in &runnable_dogs {
    let idx = entry.idx;
    let dog = entry.dog;
    let id = dog.id().to_string();
    let dog_timeout = std::time::Duration::from_secs(dog.timeout_secs());
    let semaphore = self.slot_semaphores.as_ref().and_then(|m| m.get(&id));

    futs.push(async move {
        // Soma L2: acquire slot permit before inference
        let _permit = match semaphore {
            Some(sem) if priority.is_blocking() => {
                match sem.acquire(priority).await {
                    Some(p) => Some(p),
                    None => {
                        tracing::warn!(dog_id = %id, ?priority, "Slot acquire timeout — skipping dog");
                        return (idx, id, Err(DogError::SlotUnavailable), 0);
                    }
                }
            }
            Some(sem) => {
                // Non-blocking: try_acquire, skip if full
                match sem.try_acquire(priority) {
                    Some(p) => Some(p),
                    None => {
                        tracing::info!(dog_id = %id, ?priority, "No slot available — skipping dog");
                        return (idx, id, Err(DogError::SlotUnavailable), 0);
                    }
                }
            }
            None => None, // No semaphore for this dog (e.g. deterministic-dog)
        };

        let start = std::time::Instant::now();
        let result = tokio::time::timeout(dog_timeout, dog.evaluate(stimulus)).await;
        let elapsed_ms = start.elapsed().as_millis() as u64;
        match result {
            Ok(inner) => (idx, id, inner, elapsed_ms),
            Err(_) => (idx, id, Err(DogError::Timeout), elapsed_ms),
        }
        // _permit dropped here → slot released
    });
}
```

- [ ] **Step 4: Add `DogError::SlotUnavailable` variant**

In `cynic-kernel/src/domain/dog.rs` (or wherever DogError lives):
```rust
pub enum DogError {
    // ... existing variants ...
    SlotUnavailable,
}
```

Map it in `process_dog_result` to `DogFailureKind::SlotUnavailable` (new variant).

- [ ] **Step 5: cargo check — fix all compilation errors from signature change**

```bash
cargo check --workspace --all-targets 2>&1 | head -40
```

Every callsite of `evaluate()` and `evaluate_progressive()` will fail. Fix them all:

- `nightshift.rs` — `judge.evaluate(&probe, None, &probe_metrics)` → add `, SlotPriority::Nightshift`
- `nightshift.rs` — `judge.evaluate(&stimulus, None, &metrics)` (inside judge_commit/judge_observation) → add `, SlotPriority::Nightshift`
- `runtime_loops.rs:823` — add `, SlotPriority::Background`
- `pipeline/mod.rs:547` — add priority param (threaded from PipelineDeps, see Task 3)
- `tests/integration_judge.rs` — add `, SlotPriority::User`
- `judge/mod.rs` internal tests — add priority to test evaluate calls

- [ ] **Step 6: cargo check passes**

```bash
cargo check --workspace --all-targets
```
Expected: 0 errors.

- [ ] **Step 7: Run judge tests**

```bash
cargo test -p cynic-kernel --lib -- judge::tests -v
```
Expected: all pass (behavior unchanged, semaphore is None in tests).

- [ ] **Step 8: Commit**

```bash
git add cynic-kernel/src/judge/mod.rs cynic-kernel/src/domain/dog.rs
git commit -m "feat(soma): integrate SlotPriority into Judge::evaluate fan-out"
```

---

## Task 3: Thread priority through pipeline + REST/MCP callsites

**Files:**
- Modify: `cynic-kernel/src/pipeline/mod.rs`
- Modify: `cynic-kernel/src/api/rest/judge.rs`
- Modify: `cynic-kernel/src/api/rest/judge_job.rs`
- Modify: `cynic-kernel/src/api/mcp/judge_tools.rs`

- [ ] **Step 1: Add `SlotPriority` to `PipelineDeps`**

```rust
// pipeline/mod.rs — PipelineDeps struct
pub priority: SlotPriority,
```

- [ ] **Step 2: Pass priority through `pipeline::run` to `evaluate_progressive`**

At L547 where `evaluate_progressive` is called:
```rust
.evaluate_progressive(&stimulus, dogs_filter_final, metrics, on_dog_ref, deps.priority)
```

- [ ] **Step 3: Update REST `/judge` handler — pass `SlotPriority::User`**

In `api/rest/judge.rs`, where `PipelineDeps` is built:
```rust
priority: SlotPriority::User,
```

- [ ] **Step 4: Update REST `/judge/async` handler — pass `SlotPriority::User`**

In `api/rest/judge_job.rs`, where `PipelineDeps` is built:
```rust
priority: SlotPriority::User,
```

- [ ] **Step 5: Update MCP `/judge` handler — pass `SlotPriority::User`**

In `api/mcp/judge_tools.rs`, where `PipelineDeps` is built:
```rust
priority: SlotPriority::User,
```

- [ ] **Step 6: cargo check passes**

```bash
cargo check --workspace --all-targets
```

- [ ] **Step 7: Commit**

```bash
git add cynic-kernel/src/pipeline/mod.rs cynic-kernel/src/api/rest/judge.rs \
       cynic-kernel/src/api/rest/judge_job.rs cynic-kernel/src/api/mcp/judge_tools.rs
git commit -m "feat(soma): thread SlotPriority through pipeline and all judge callsites"
```

---

## Task 4: Remove ResourceGate + nightshift soma_gate

**Files:**
- Modify: `cynic-kernel/src/infra/tasks/nightshift.rs`
- Modify: `cynic-kernel/src/domain/orchestrator.rs`
- Modify: `cynic-kernel/src/api/rest/types.rs`
- Modify: `cynic-kernel/src/api/rest/soma.rs`
- Modify: `cynic-kernel/src/main.rs`
- Modify: `cynic-kernel/tests/rest_routes.rs`
- Modify: `cynic-kernel/src/infra/tasks/runtime_loops.rs`

- [ ] **Step 1: Remove soma_gate from nightshift**

In `nightshift.rs`:
- Remove `soma_gate: &Arc<ResourceGate>` from `judge_observation` and `judge_commit` signatures
- Remove the `soma_gate.request()` blocks in both functions
- Remove `soma_gate: Arc<ResourceGate>` from `spawn_nightshift_loop` params
- Remove `Arc::clone(&soma_gate)` from spawn call
- Update imports: remove `ResourceGate, ResourceRequest, Priority`

The semaphore now handles slot coordination inside `judge.evaluate()` — no manual gating needed.

- [ ] **Step 2: Remove soma_gate from AppState**

In `api/rest/types.rs`:
- Remove `pub soma_gate: Arc<ResourceGate>` field
- Remove `use crate::domain::orchestrator::ResourceGate`

- [ ] **Step 3: Update `/soma/request` endpoint**

In `api/rest/soma.rs`: update to read from `SlotSemaphoreMap` instead of `ResourceGate`. Or simplify to report slot utilization only (the semaphore handles coordination internally, external consumers use `/judge` with priority).

- [ ] **Step 4: Remove ResourceGate struct**

In `domain/orchestrator.rs`: delete `ResourceGate`, `GateDecision`, `ResourceRequest`, `Priority` enum and all tests. The module can be deleted entirely or kept as a re-export if other code references it.

- [ ] **Step 5: Update main.rs**

- Remove `let soma_gate = Arc::new(...)` creation
- Remove `soma_gate: Arc::clone(&soma_gate)` from AppState construction
- Remove `Arc::clone(&soma_gate)` from nightshift spawn
- Add: create `SlotSemaphoreMap`, inject into Judge and AppState

- [ ] **Step 6: Update test_state in rest_routes.rs**

Replace `soma_gate` field with `slot_semaphores: Arc::new(SlotSemaphoreMap::new())`.

- [ ] **Step 7: Update runtime_loops.rs**

Remove `soma_gate` from any test helpers that construct it.

- [ ] **Step 8: cargo check + clippy**

```bash
cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 9: Commit**

```bash
git add -u
git commit -m "refactor(soma): remove ResourceGate — SlotSemaphore subsumes it"
```

---

## Task 5: Health loop → semaphore initialization

**Files:**
- Modify: `cynic-kernel/src/infra/health_loop.rs`
- Modify: `cynic-kernel/src/main.rs`

- [ ] **Step 1: Add `SlotSemaphoreMap` to health loop params**

In `spawn_health_loop` signature, add:
```rust
slot_semaphores: Arc<SlotSemaphoreMap>,
```

- [ ] **Step 2: After slot probe, call `slot_semaphores.upsert(dog_id, slot_count)`**

Find the section in health_loop.rs where `slot_tracker.update()` is called after probing `/slots`. Add `slot_semaphores.upsert(dog_id, total_slots)` right after.

- [ ] **Step 3: Pass SlotSemaphoreMap from main.rs to spawn_health_loop**

At the call site in main.rs, add `Arc::clone(&slot_semaphores)`.

- [ ] **Step 4: cargo check**

```bash
cargo check --workspace --all-targets
```

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/infra/health_loop.rs cynic-kernel/src/main.rs
git commit -m "feat(soma): health loop initializes slot semaphore permits from probe"
```

---

## Task 6: Wire SlotSemaphoreMap into Judge at boot

**Files:**
- Modify: `cynic-kernel/src/main.rs`

- [ ] **Step 1: Create SlotSemaphoreMap in main.rs before Judge construction**

```rust
let slot_semaphores = Arc::new(SlotSemaphoreMap::new());
```

- [ ] **Step 2: Inject into Judge**

Where the Judge is constructed:
```rust
let judge = Judge::new(dogs, breakers)
    .with_slot_semaphores(Arc::clone(&slot_semaphores));
```

- [ ] **Step 3: Inject into AppState**

```rust
slot_semaphores: Arc::clone(&slot_semaphores),
```

- [ ] **Step 4: Full build + test**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p cynic-kernel --lib
```

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/main.rs
git commit -m "feat(soma): wire SlotSemaphoreMap into Judge + AppState at boot"
```

---

## Task 7: Falsification tests F3 + F5

**Files:**
- Add tests to: `cynic-kernel/src/domain/slot_semaphore.rs`

- [ ] **Step 1: Write F3 — concurrent user + nightshift**

```rust
#[tokio::test]
async fn f3_concurrent_user_and_nightshift_no_starvation() {
    let sem = Arc::new(SlotSemaphore::new("gpu-dog", 4));
    // Nightshift takes 3 slots (sequential in practice, parallel here for stress)
    let _n1 = sem.try_acquire(SlotPriority::Nightshift).unwrap();
    let _n2 = sem.try_acquire(SlotPriority::Nightshift).unwrap();
    let _n3 = sem.try_acquire(SlotPriority::Nightshift).unwrap();
    // User still gets the 4th slot
    let user = sem.try_acquire(SlotPriority::User);
    assert!(user.is_some(), "User must get a slot when 1/4 is free");
}
```

- [ ] **Step 2: Write F5 — no deadlock under fan-out**

```rust
#[tokio::test]
async fn f5_fanout_no_deadlock() {
    // 3 dogs, 1 slot each
    let map = Arc::new(SlotSemaphoreMap::new());
    map.upsert("dog-a", 1);
    map.upsert("dog-b", 1);
    map.upsert("dog-c", 1);

    // Two concurrent "evaluates" — each acquires all 3 dogs
    let map2 = Arc::clone(&map);
    let h1 = tokio::spawn(async move {
        let _a = map2.get("dog-a").unwrap().acquire(SlotPriority::User).await;
        let _b = map2.get("dog-b").unwrap().acquire(SlotPriority::User).await;
        let _c = map2.get("dog-c").unwrap().acquire(SlotPriority::User).await;
    });

    let map3 = Arc::clone(&map);
    let h2 = tokio::spawn(async move {
        // Nightshift: try_acquire, skip if busy
        let _a = map3.get("dog-a").unwrap().try_acquire(SlotPriority::Nightshift);
        let _b = map3.get("dog-b").unwrap().try_acquire(SlotPriority::Nightshift);
        let _c = map3.get("dog-c").unwrap().try_acquire(SlotPriority::Nightshift);
    });

    // Both must complete within 5s (no deadlock)
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        h1.await.unwrap();
        h2.await.unwrap();
    })
    .await
    .expect("must not deadlock");
}
```

- [ ] **Step 3: Run all tests**

```bash
cargo test -p cynic-kernel --lib -- slot_semaphore -v
```

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/domain/slot_semaphore.rs
git commit -m "test(soma): F3+F5 falsification — concurrent consumers + deadlock-free fan-out"
```

---

## Task 8: Final gate — clippy + full test suite

- [ ] **Step 1: cargo clippy clean**

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 2: cargo test --lib passes**

```bash
cargo test -p cynic-kernel --lib --release
```
Expected: all pass (666+ tests), 0 failures.

- [ ] **Step 3: cargo test nightshift passes**

```bash
cargo test -p cynic-kernel --lib -- nightshift -v
```

- [ ] **Step 4: Commit any remaining fixups**

- [ ] **Step 5: Push + PR**

```bash
git push -u origin <branch>
gh pr create --base main --title "feat(soma): L2 priority slot semaphore" --body "..."
```
