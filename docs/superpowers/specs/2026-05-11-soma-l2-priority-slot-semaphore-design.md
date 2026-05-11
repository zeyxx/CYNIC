# Soma L2 — Priority Slot Semaphore

> Per-dog priority-aware semaphore for inference slot coordination across all consumers.

**Date:** 2026-05-11
**Status:** Design approved, pending implementation
**Replaces:** ResourceGate (Soma L3 — static threshold check)
**Falsify:** 2 concurrent consumers (nightshift + user) complete without timeout starvation on 4 slots.

---

## Problem

The organism has 4 inference consumers competing for the same physical llama-server slots:

| Consumer | Pattern | Priority | Behavior |
|----------|---------|----------|----------|
| User `/judge` | burst, unpredictable | highest | must get a result |
| Hermes agent | periodic, long-running | high | must get a result |
| Nightshift | every 4h, 30+ sequential evals | low | can skip |
| Crystal challenge | every 5min, re-judges crystals | lowest | can skip |

Current state (Soma L3 — `ResourceGate`): binary check-and-skip. No real queue, no priority ordering, no coordination between consumers. Nightshift Phase 2 was ungated entirely (fixed in PR#136).

With all consumers active on qwen35-9b-gpu (4 slots): 4 consumers x 1 slot each = saturation. User waits with no priority guarantee.

### Observed load profile (this boot, ~6.5h)

- `qwen35-9b-gpu`: 2157 req, 13.3s avg, 30.6% slot occupancy (4 slots)
- `gemma-4-e4b-gpu`: 15 req, 32.8s avg, 0.5% occupancy (4 slots)
- `qwen25-7b-core`: 1 req (failed), effectively dead
- All traffic was user `/judge` (hackathon). No background consumers active.

Contention pattern: periodic background bursts (nightshift ~6.5min, crystal challenge ~frequent) colliding with unpredictable user requests.

---

## Design

### Core: `SlotSemaphore` (one per dog)

```rust
pub struct SlotSemaphore {
    dog_id: String,
    /// Tokio semaphore — N permits = N physical llama-server slots.
    permits: tokio::sync::Semaphore,
    /// Priority queue of waiters. Highest priority served first when a permit frees.
    waiters: Mutex<BinaryHeap<PriorityWaiter>>,
    /// Current total slots (for health reporting / dynamic resize).
    total_slots: AtomicU32,
}
```

### Priority levels

```rust
pub enum SlotPriority {
    Background = 0,
    Nightshift = 1,
    Hermes = 2,
    User = 3,
}
```

### Acquire behavior (mixed: wait or skip)

| Priority | Method | Timeout | On failure |
|----------|--------|---------|------------|
| User | `acquire(timeout=30s)` | 30s | 503 to caller |
| Hermes | `acquire(timeout=15s)` | 15s | reschedule task |
| Nightshift | `try_acquire()` | instant | skip eval, continue cycle |
| Background | `try_acquire()` | instant | skip eval |

Higher-priority waiters are served first when a permit frees (priority queue, not FIFO).

### RAII permit guard

```rust
pub struct SlotPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
    dog_id: String,
    acquired_at: Instant,
    priority: SlotPriority,
}
// Drop → releases permit → wakes highest-priority waiter
```

### Initialization

- Boot: 0 permits per dog (no consumers active yet)
- Health loop first probe (~30s): `add_permits(N)` where N = discovered slot count
- Nightshift warmup = 60s, so permits arrive before any background consumer
- User requests during first 30s: rare (boot time), would wait then timeout at 30s

### Dynamic resize

- Health loop discovers slot count changes → `add_permits(delta)` if slots increase
- If slots decrease: no `remove_permits` needed — excess permits drain naturally as current holders release
- Dog discovered/expired: create/remove semaphore entry

---

## Integration point: `Judge::evaluate`

All inference consumers go through `Judge::evaluate`. The fan-out to each dog is where permits are acquired.

### Signature change

```rust
// Before:
pub async fn evaluate(&self, stimulus: &Stimulus, filter: Option<&[&str]>, metrics: &Metrics) -> Result<Verdict>

// After:
pub async fn evaluate(&self, stimulus: &Stimulus, filter: Option<&[&str]>, metrics: &Metrics, priority: SlotPriority) -> Result<Verdict>
```

### Fan-out with permits

```
evaluate(stimulus, filter, metrics, priority):
    for each dog (parallel futures):
        // Existing gates: circuit breaker, quality gate, budget, K22
        ...
        // NEW: slot permit
        permit = match priority.is_blocking() {
            true  => semaphore[dog_id].acquire(priority, timeout).await
            false => semaphore[dog_id].try_acquire(priority)
        }
        if permit.is_err():
            skip this dog (same as circuit-open behavior)
            continue
        score = dog.score(stimulus).await
        drop(permit)  // RAII release
```

Key properties:
- Each dog acquires its permit independently — no cross-dog locking → no deadlock
- A slow dog doesn't block other dogs in the same evaluate
- If a dog refuses the permit (try_acquire fail), that dog is excluded from the verdict — same behavior as circuit-open today
- Permit is held for exactly the duration of `dog.score()` — accurate in-flight tracking

### Callsite changes

| Callsite | Before | After |
|----------|--------|-------|
| REST `/judge` | `evaluate(stim, filter, metrics)` | `evaluate(stim, filter, metrics, SlotPriority::User)` |
| MCP `/judge` | `evaluate(stim, filter, metrics)` | `evaluate(stim, filter, metrics, SlotPriority::User)` |
| REST `/judge/async` | `pipeline::run(...)` | pass `SlotPriority::User` through pipeline |
| Nightshift `judge_commit` | `soma_gate.request()` + `evaluate()` | `evaluate(stim, None, metrics, SlotPriority::Nightshift)` |
| Nightshift `judge_observation` | `soma_gate.request()` + `evaluate()` | `evaluate(stim, None, metrics, SlotPriority::Nightshift)` |
| Crystal challenge | disabled, future | `evaluate(stim, None, metrics, SlotPriority::Background)` |

### What gets removed

- `ResourceGate` struct and all its tests (subsumes by semaphore)
- `soma_gate` field from `AppState`
- `soma_gate.request()` calls in nightshift (both phases)
- `POST /soma/request` REST endpoint (external consumers use priority in `/judge` request)

---

## State ownership

```
AppState {
    slot_semaphores: Arc<SlotSemaphoreMap>,  // BTreeMap<dog_id, SlotSemaphore>
}

Judge {
    slot_semaphores: Arc<SlotSemaphoreMap>,  // injected at construction
}
```

Health loop is the sole writer (creates semaphores, resizes permits). Judge is the sole reader/acquirer. SlotTracker remains for `/health` reporting but is no longer the coordination mechanism.

---

## Falsification tests

### F1: Permit exhaustion
4 slots on a dog. 4 concurrent `try_acquire(Background)` → all succeed. 5th `try_acquire(Background)` → fail (None).

### F2: Priority ordering
4 slots busy (background). 1 `acquire(User, 30s)` waits. 1 background finishes → User gets the slot (not another background). Assert: User wait time < 15s (one eval duration).

### F3: Concurrent consumers (TODO.md criterion)
4 slots. Nightshift eval + User eval concurrent. Both complete without timeout. Nightshift uses try_acquire (may skip some dogs). User uses acquire (waits if needed, gets result).

### F4: Boot initialization
0 permits at start. `acquire(User, 1s)` → timeout (no permits). `add_permits(4)`. `acquire(User, 1s)` → success.

### F5: No deadlock under fan-out
3 dogs, 1 slot each. 2 concurrent evaluates (User + Nightshift). Each evaluate acquires permits on all 3 dogs in parallel. No deadlock because: (a) no cross-dog dependency, (b) nightshift uses try_acquire (never holds-and-waits).

---

## Scope boundaries

**In scope:**
- `SlotSemaphore` struct with priority queue
- Integration into `Judge::evaluate`
- Health loop permit initialization and resize
- Remove `ResourceGate` and manual soma gate checks
- Unit tests for F1-F5

**Out of scope (future):**
- Preemption (cancel in-flight low-priority evals) — not needed while max eval time ~13s
- External consumer REST API (Hermes calls `/judge` with priority header, not `/soma/request`)
- Observation emission for soma decisions (K15 audit trail)
- Crystal challenge re-enablement (separate PR after L2 lands)

---

## Risk

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Priority inversion (low-priority holds slot, high-priority waits) | Certain (by design) | Max wait = 1 eval duration (~13s). Option A: acceptable. |
| Permit leak (acquire without release) | Low | RAII guard. Timeout on acquire as safety net. |
| Health loop dies → no resize | Low | Semaphore works with initial permits. SlotTracker staleness already detected. |
| tokio::Semaphore doesn't support priority | Certain | Custom wrapper with BinaryHeap of oneshot waiters. |
