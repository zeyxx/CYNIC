# Soma L5: Inference Scheduler — Priority Queue with Aging

**Date:** 2026-05-10  
**Status:** Design (reviewed, concurrency fixes applied)  
**Falsification:** 3 concurrent consumers on 1 slot, all complete within bounded time (no infinite starvation).

---

## Problem

Current Soma (L1-L4) is advisory only:
- `ResourceGate.request()` returns `Allocate` or `Queue` but nothing enforces it
- No actual queue — Nightshift drops work, Hermes polls blindly (503 → retry 5s)
- No reservation — two callers can both receive `Allocate` simultaneously (race)
- Designed for fixed 3 consumers; doesn't scale to N

Result: consumers are unstable, starvation prevents bringing more online.

## Design: InferenceScheduler

A **priority semaphore with aging** that replaces advisory gating with actual blocking acquisition.

### Core Semantics

```
permits = total_slots (dynamic, reconciled by health loop)
acquire(consumer, priority, timeout) → SlotLease | Timeout | BackendUnavailable
release(lease) → wakes next waiter by effective_priority
```

### Priority with Aging

```
effective_priority(waiter) = base_priority as u32 + (wait_secs / AGING_RATE)
```

- `base_priority`: u8 (0-255), set by consumer at acquire time
- `effective_priority`: computed as **u32** (no overflow — saturates naturally at practical bounds)
- `AGING_RATE`: configurable (default: 15s per +1 priority point)
- Effect: a priority-0 consumer waiting 120s → effective 8. A priority-10 consumer arriving now → effective 10. After 30 more seconds → the aged waiter wins (effective 10 vs 10, FIFO breaks tie).

**Starvation bound:** Any consumer is guaranteed service within `AGING_RATE × (max_priority_gap) + slot_duration` seconds. With defaults: 15 × 255 + 60 ≈ 64 min (theoretical worst case with 255 priority levels and continuous higher-priority arrivals).

**Practical bound:** With priority gap ≤ 50 and typical slot durations (30-120s): max wait ≈ 15 × 50 + 120 = 870s ≈ 15 min.

### Data Structures

```rust
// domain/scheduler.rs (NEW)

/// Single Mutex protects ALL mutable state — eliminates TOCTOU races.
/// Critical section is microseconds (no async, no I/O inside lock).
pub struct InferenceScheduler {
    inner: Mutex<SchedulerState>,
    /// Next lease ID (monotonic, lock-free)
    next_lease_id: AtomicU64,
    /// Config (immutable after construction)
    config: SchedulerConfig,
}

struct SchedulerState {
    /// Total available slots (updated by reconcile)
    total_slots: u32,
    /// Currently held permits
    held: u32,
    /// Waiting consumers (small N ≤ 16, linear scan)
    waiters: Vec<Waiter>,
    /// Active leases for TTL enforcement + observability
    leases: BTreeMap<u64, LeaseInfo>,
}

struct Waiter {
    consumer: String,
    base_priority: u8,
    enqueued_at: Instant,
    tx: tokio::sync::oneshot::Sender<Result<SlotLease, AcquireError>>,
}

pub struct SchedulerConfig {
    pub aging_rate_secs: u32,
    pub lease_ttl: Duration,
    pub max_queue_depth: usize,
}

/// RAII lease — auto-releases on Drop for in-process callers.
/// REST callers use explicit POST /soma/release (no Drop, TTL is the safety net).
pub struct SlotLease {
    pub id: u64,
    pub consumer: String,
    pub acquired_at: Instant,
    /// If Some, Drop calls release. None for REST-managed leases.
    scheduler: Option<Arc<InferenceScheduler>>,
}

impl Drop for SlotLease {
    fn drop(&mut self) {
        if let Some(sched) = self.scheduler.take() {
            sched.release_by_id(self.id);
        }
    }
}

pub enum AcquireError {
    Timeout { queue_depth: usize },
    BackendUnavailable,
    QueueFull,
}

struct LeaseInfo {
    consumer: String,
    acquired_at: Instant,
    expires_at: Instant,
}
```

### Algorithm

All operations acquire the **single `inner` Mutex**. Critical section contains only in-memory operations (no `.await`, no I/O). Lock held for < 1μs even with 16 waiters.

**acquire(consumer, priority, timeout):**
```
1. Lock inner.
2. If total_slots == 0 → return Err(BackendUnavailable).  // K14: don't wait on dead backend
3. If held < total_slots → held += 1, create lease, insert into leases, return Ok(lease).
4. If waiters.len() >= max_queue_depth → return Err(QueueFull).
5. Create oneshot::channel(). Insert Waiter into vec. Drop lock.
6. tokio::time::timeout(timeout, rx).await:
   - Ok(Ok(lease)) → return Ok(lease).
   - Ok(Err(e)) → return Err(e).  // BackendUnavailable from reconcile(0)
   - Err(elapsed) → Lock inner, remove self from waiters (by tx identity), return Err(Timeout).
```

**Cancel safety:** If the caller future is dropped (connection closed), the oneshot `rx` is dropped. On next `release()`, `tx.send()` returns `Err`. The release loop retries with next waiter (see below).

**release_by_id(lease_id):**
```
1. Lock inner.
2. Remove lease from leases. Decrement held.
3. wake_next_waiter(&mut state).  // helper, called under same lock
```

**wake_next_waiter(state):** (called with lock held)
```
loop {
    If waiters is empty → return (permit stays available for next acquire).
    Compute effective_priority for each waiter. Pick max (FIFO on tie by enqueued_at).
    Remove winner from vec.
    Attempt tx.send(Ok(new_lease)):
      - Ok → held += 1, insert lease into leases, return.
      - Err (rx dropped = cancelled caller) → discard waiter, loop again.
}
```

**reconcile(new_total_slots):** (called by health loop every 30s)
```
1. Lock inner.
2. state.total_slots = new_total_slots.
3. If new_total_slots == 0:
   - Drain ALL waiters, send Err(BackendUnavailable) to each.
   - Return.
4. While held < total_slots AND waiters non-empty:
   - wake_next_waiter(&mut state).
```

**expire_leases():** (called by background task every 10s)
```
1. Lock inner.
2. Collect expired lease IDs: leases where now > expires_at.
3. For each expired:
   - Remove from leases. Decrement held. Log warning.
   - wake_next_waiter(&mut state).
```

Note: No re-entrancy issue — `expire_leases` and `wake_next_waiter` operate on the same `&mut SchedulerState` reference under a single lock acquisition.

### Integration Points

| Consumer | Current | New |
|----------|---------|-----|
| **Judge** (`runnable_dogs`) | `tracker.all_slots_busy()` → skip Dog | **Keep SlotTracker per-Dog check** (prevents per-backend overload). Additionally: `scheduler.try_acquire(dog_id, 200, Duration::ZERO)` for global permit tracking. Release after Dog eval. Both checks must pass. |
| **Nightshift** (`judge_commit`) | `soma_gate.request()` → drop on Queue | `scheduler.acquire("nightshift", 128, Duration::from_secs(30)).await` — actually waits up to 30s. If timeout → skip. |
| **Inference proxy** (`/v1/chat/completions`) | `tracker.all_slots_busy()` → 503 | `scheduler.acquire(consumer_tag, priority, Duration::from_secs(timeout)).await`. Consumer from `X-Consumer` header (default: "anonymous"). Priority from `X-Priority` header (default: 128). |
| **Future organs** | N/A | Call `POST /soma/acquire` or use in-process `scheduler.acquire()`. |

**Key:** SlotTracker per-Dog check is **retained** alongside the scheduler. They are orthogonal:
- SlotTracker: "is this specific backend saturated?" (per-Dog)
- Scheduler: "is the global inference pool at capacity?" (all backends)

### REST API Changes

**Deprecate** `POST /soma/request` — returns stats only, no Allocate/Queue semantics:
```
POST /soma/request  (DEPRECATED)
→ 200 { "advisory": true, "total_slots": 2, "held": 1, "queue_depth": 0 }
```

**New endpoints:**

```
POST /soma/acquire
{
  "consumer": "hermes-search-1",
  "priority": 200,
  "timeout_secs": 30
}

→ 200 { "lease_id": 42, "consumer": "hermes-search-1", "acquired_at": "..." }
→ 408 { "error": "timeout", "queue_depth": 3 }
→ 503 { "error": "backend_unavailable" }
→ 429 { "error": "queue_full", "max_depth": 16 }
```

```
POST /soma/release
{ "lease_id": 42 }

→ 200 { "held_secs": 45 }
→ 404 { "error": "lease_not_found" }
```

```
GET /soma/status
→ 200 {
  "total_slots": 2,
  "held": 1,
  "available": 1,
  "queue_depth": 2,
  "aging_rate_secs": 15,
  "lease_ttl_secs": 60,
  "waiters": [
    { "consumer": "nightshift", "priority": 128, "waiting_secs": 12, "effective_priority": 128 }
  ],
  "leases": [
    { "lease_id": 41, "consumer": "hermes-search-1", "held_secs": 23, "ttl_remaining_secs": 37 }
  ]
}
```

### Observability (K15 compliant)

- `GET /soma/status` → full scheduler state (queue depth, waiters, leases)
- `/health` includes: `scheduler_queue_depth`, `scheduler_held`, `scheduler_total_slots`
- Warnings emitted (tracing::warn) on:
  - Lease TTL expiry (consumer didn't release — potential bug in consumer)
  - Queue depth > 5 (contention building)
  - Starvation: any waiter > 5 minutes (should not happen with aging)
  - `reconcile(0)` → all waiters drained (backend went offline)

### Config

```toml
# In backends.toml or kernel config
[scheduler]
aging_rate_secs = 15         # seconds of wait = +1 effective priority
lease_ttl_secs = 60          # auto-release after this
max_queue_depth = 16         # reject new waiters beyond this (429)
```

### Concurrency Model

**Single `Mutex<SchedulerState>`** protects all mutable state:
- `total_slots`, `held`, `waiters`, `leases` — all under one lock
- No TOCTOU: check-and-modify is atomic within the lock
- No deadlock: only one lock, never held across `.await`
- Lock duration: O(N) where N = waiters.len() ≤ 16 — worst case ~1μs

**oneshot channels:** Each waiter gets a `oneshot::Sender`. The `release` path sends on it. If the receiver was dropped (cancelled future), `send()` fails and we retry next waiter.

**Background task:** `tokio::spawn` with `tokio::time::interval(10s)` for lease expiry. Acquires same Mutex — no separate lock.

**RAII for in-process callers:** `SlotLease` holds `Option<Arc<InferenceScheduler>>`. On Drop → auto-release. Prevents permit leak on panic/early-return. REST callers get leases without the Arc (TTL is their safety net).

### Clock Injection (for testing)

```rust
pub struct InferenceScheduler {
    inner: Mutex<SchedulerState>,
    next_lease_id: AtomicU64,
    config: SchedulerConfig,
    /// Injectable clock for deterministic aging tests.
    /// Production: Instant::now. Tests: tokio::time::Instant (with pause).
    clock: Box<dyn Fn() -> Instant + Send + Sync>,
}
```

Tests use `tokio::time::pause()` + `tokio::time::advance()` to deterministically test aging without real sleeps.

### Files to Create/Modify

| File | Action |
|------|--------|
| `src/domain/scheduler.rs` | **NEW** — InferenceScheduler, SlotLease, AcquireError |
| `src/domain/mod.rs` | Add `pub mod scheduler;` |
| `src/domain/orchestrator.rs` | Deprecate GateDecision. Keep ResourceGate for `/soma/request` compat only. |
| `src/api/rest/soma.rs` | Add `/soma/acquire`, `/soma/release`, `/soma/status`. Modify `/soma/request` to return advisory stats. |
| `src/api/rest/inference_proxy.rs` | Replace `tracker.all_slots_busy()` with `scheduler.acquire()` |
| `src/judge/mod.rs` | Add `scheduler.try_acquire()` alongside existing `tracker.all_slots_busy()` |
| `src/infra/tasks/nightshift.rs` | Replace `soma_gate.request()` with `scheduler.acquire().await` |
| `src/infra/health_loop.rs` | Call `scheduler.reconcile(total_free)` after slot probe |
| `src/infra/boot.rs` | Instantiate `InferenceScheduler`, spawn lease expiry task, wire into AppState |
| `src/api/rest/types.rs` | Add `scheduler: Arc<InferenceScheduler>` to AppState |

### Tests

1. **Unit: priority ordering** — 3 waiters (priority 0, 128, 255), 1 slot frees → priority 255 gets it.
2. **Unit: aging** — `tokio::time::pause()` + `advance()`. Priority-0 waiter + 150s elapsed → effective 10. Priority-10 waiter just arrived → effective 10. FIFO tie: aged waiter wins.
3. **Unit: lease TTL** — acquire, advance time past TTL → expire_leases fires → next waiter wakes.
4. **Unit: reconcile up** — total_slots 1→2 while 1 waiter queued → waiter immediately wakes.
5. **Unit: reconcile down to 0** — all waiters receive `Err(BackendUnavailable)`.
6. **Unit: cancel safety** — drop rx before release → release retries next waiter, no permit leak.
7. **Unit: RAII Drop** — Drop SlotLease without explicit release → permit freed.
8. **Unit: queue full** — 17th waiter receives `Err(QueueFull)`.
9. **Integration: 3 consumers, 1 slot** — spawn 3 tasks (priority 200, 128, 64), 1 slot. All complete. Order: 200→128→64.
10. **Integration: starvation bound** — priority-0 consumer alongside continuous priority-200 arrivals (every 5s for 2 min). Verify aged consumer gets served before 64 min.

### Non-Goals

- **Per-backend queuing**: V1 is a global pool. Per-backend saturation is handled by SlotTracker (retained).
- **Preemption**: No killing in-progress inference. Leases respected until completion/TTL.
- **Persistence**: In-memory. Reboot = empty (health loop reconciles in 30s).
- **Cross-kernel distribution**: Single kernel instance.

---

## Falsification Criteria

1. **Primary:** 3 concurrent `acquire()` on 1-slot → all 3 complete within `3 × lease_ttl` (180s). None returns Timeout with default 60s timeout.
2. **Aging:** A priority-0 consumer queued > `aging_rate × priority_gap` seconds → served before newly arriving same-effective-priority consumers (FIFO tie-break).
3. **Lease safety:** Consumer dies (Drop) without explicit release → lease auto-freed via RAII (in-process) or TTL (REST) → next waiter wakes → no deadlock.
4. **Reconcile down:** Backend goes offline → `reconcile(0)` → all waiters receive `BackendUnavailable` immediately (not after timeout).
5. **No double-allocation:** Under Mutex, only one caller can increment `held` per available slot. Falsify: run 100 concurrent acquires on 1 slot → exactly 1 gets lease, 99 queue or timeout.
