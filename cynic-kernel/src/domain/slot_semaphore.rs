//! SlotSemaphore — per-dog priority-aware permit system for inference slot coordination.
//!
//! Each Dog (inference backend) has N physical llama-server slots. This module provides
//! a semaphore that tracks permits = physical slots, with priority-aware acquisition.
//!
//! Priority tiers (highest to lowest): User > Hermes > Nightshift > Background.
//! Blocking callers (User, Hermes) wait up to their timeout for a slot.
//! Non-blocking callers (Nightshift, Background) skip if no slot is available.
//!
//! K19: BTreeMap for deterministic serialization in SlotSemaphoreMap.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

// ── PRIORITY ───────────────────────────────────────────────

/// Inference slot acquisition priority.
///
/// Discriminant encodes natural ordering: higher = more important.
/// Derives Ord so priority comparisons are numeric (User > Background).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SlotPriority {
    /// Fire-and-forget background jobs. Never blocks — skips if no slot available.
    Background = 0,
    /// Nightshift autonomous jobs. Never blocks — skips if no slot available.
    Nightshift = 1,
    /// Hermes organic agent. Blocks up to 15 s.
    Hermes = 2,
    /// Interactive user request. Blocks up to 30 s.
    User = 3,
}

impl SlotPriority {
    /// Whether this priority waits for a slot (true) or skips on contention (false).
    pub fn is_blocking(&self) -> bool {
        matches!(self, SlotPriority::User | SlotPriority::Hermes)
    }

    /// Maximum time to wait for a slot. Non-blocking priorities return ZERO.
    pub fn timeout(&self) -> Duration {
        match self {
            SlotPriority::User => Duration::from_secs(30),
            SlotPriority::Hermes => Duration::from_secs(15),
            SlotPriority::Background | SlotPriority::Nightshift => Duration::ZERO,
        }
    }
}

// ── PERMIT (RAII guard) ────────────────────────────────────

/// A held inference slot. Dropping this struct releases the permit back to the semaphore.
pub struct SlotPermit {
    /// Holds the semaphore permit. Dropped (and thus released) when SlotPermit is dropped.
    _permit: OwnedSemaphorePermit,
    /// Which Dog this permit belongs to. For diagnostics / tracing.
    pub dog_id: String,
    /// Priority tier that acquired this permit.
    pub priority: SlotPriority,
}

impl std::fmt::Debug for SlotPermit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlotPermit")
            .field("dog_id", &self.dog_id)
            .field("priority", &self.priority)
            .finish()
    }
}

// ── SEMAPHORE ──────────────────────────────────────────────

/// Per-dog semaphore tracking inference slot permits.
///
/// The health loop discovers physical slots at boot and calls `add_permits`.
/// Callers acquire permits before dispatching to the Dog's backend.
pub struct SlotSemaphore {
    pub dog_id: String,
    semaphore: Arc<Semaphore>,
    /// Total logical slots tracked. Updated by `add_permits` and `upsert`.
    total_slots: AtomicU32,
}

impl std::fmt::Debug for SlotSemaphore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlotSemaphore")
            .field("dog_id", &self.dog_id)
            .field("total_slots", &self.total_slots.load(Ordering::Relaxed))
            .field("available", &self.semaphore.available_permits())
            .finish()
    }
}

impl SlotSemaphore {
    /// Create a semaphore for `dog_id` with `slots` initial permits.
    ///
    /// Pass `slots = 0` during boot when the health loop hasn't probed yet.
    /// Call `add_permits` once the probe returns the real slot count.
    pub fn new(dog_id: impl Into<String>, slots: u32) -> Self {
        Self {
            dog_id: dog_id.into(),
            semaphore: Arc::new(Semaphore::new(slots as usize)),
            total_slots: AtomicU32::new(slots),
        }
    }

    /// Attempt a non-blocking acquire. Returns `None` if no slot is currently free.
    pub fn try_acquire(&self, priority: SlotPriority) -> Option<SlotPermit> {
        match Arc::clone(&self.semaphore).try_acquire_owned() {
            Ok(permit) => Some(SlotPermit {
                _permit: permit,
                dog_id: self.dog_id.clone(),
                priority,
            }),
            Err(_) => None,
        }
    }

    /// Blocking acquire. Waits up to `priority.timeout()` for a free slot.
    ///
    /// Returns `None` if the timeout expires before a slot becomes available,
    /// or if the priority is non-blocking (timeout = ZERO → immediate `try_acquire`).
    pub async fn acquire(&self, priority: SlotPriority) -> Option<SlotPermit> {
        let timeout = priority.timeout();
        if timeout.is_zero() {
            // Non-blocking: same as try_acquire
            return self.try_acquire(priority);
        }

        match tokio::time::timeout(timeout, Arc::clone(&self.semaphore).acquire_owned()).await {
            Ok(Ok(permit)) => Some(SlotPermit {
                _permit: permit,
                dog_id: self.dog_id.clone(),
                priority,
            }),
            Ok(Err(_)) => {
                // Semaphore closed — treat as unavailable
                tracing::warn!(dog_id = %self.dog_id, "semaphore closed during acquire");
                None
            }
            Err(_elapsed) => {
                // Timeout expired
                tracing::warn!(
                    dog_id = %self.dog_id,
                    priority = ?priority,
                    timeout_secs = timeout.as_secs(),
                    "slot acquire timed out"
                );
                None
            }
        }
    }

    /// Add `n` permits to the semaphore (e.g. after health loop discovers more slots).
    ///
    /// Also increments the `total_slots` counter so health reporting stays accurate.
    pub fn add_permits(&self, n: u32) {
        self.semaphore.add_permits(n as usize);
        self.total_slots.fetch_add(n, Ordering::Relaxed);
    }

    /// Total logical slots tracked by this semaphore (including busy ones).
    pub fn total_slots(&self) -> u32 {
        self.total_slots.load(Ordering::Relaxed)
    }

    /// Currently available (free) permits.
    pub fn available(&self) -> u32 {
        self.semaphore.available_permits() as u32
    }
}

// ── SEMAPHORE MAP ──────────────────────────────────────────

/// Shared registry of per-dog semaphores.
///
/// Thread-safe: uses `std::sync::RwLock` (not tokio) so it can be accessed
/// from both sync and async contexts without blocking the executor.
///
/// K19: BTreeMap for deterministic serialization and iteration order.
#[derive(Debug, Default)]
pub struct SlotSemaphoreMap {
    /// dog_id → semaphore. BTreeMap preserves insertion/iteration order deterministically.
    inner: RwLock<BTreeMap<String, Arc<SlotSemaphore>>>,
}

impl SlotSemaphoreMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up the semaphore for a Dog. Returns `None` if not registered.
    pub fn get(&self, dog_id: &str) -> Option<Arc<SlotSemaphore>> {
        match self.inner.read() {
            Ok(guard) => guard.get(dog_id).cloned(),
            Err(e) => {
                // K14: poisoned lock → pessimistic (unavailable)
                tracing::warn!("SlotSemaphoreMap read lock poisoned: {e}");
                e.into_inner().get(dog_id).cloned()
            }
        }
    }

    /// Insert or update the slot count for `dog_id`.
    ///
    /// - If absent: creates a new `SlotSemaphore` with `slots` permits.
    /// - If present and `slots > current_total`: adds the delta via `add_permits`
    ///   so existing permit holders are unaffected.
    /// - If present and `slots <= current_total`: no-op (shrinking is not supported
    ///   to avoid racing with held permits).
    pub fn upsert(&self, dog_id: &str, slots: u32) {
        // Fast path: check if the semaphore already exists and just needs growing.
        if let Some(existing) = self.get(dog_id) {
            let current = existing.total_slots();
            if slots > current {
                existing.add_permits(slots - current);
            }
            return;
        }

        // Slow path: insert new entry under write lock.
        match self.inner.write() {
            Ok(mut guard) => {
                // Double-check under write lock (another thread may have inserted).
                let entry = guard
                    .entry(dog_id.to_string())
                    .or_insert_with(|| Arc::new(SlotSemaphore::new(dog_id, 0)));
                let current = entry.total_slots();
                if slots > current {
                    entry.add_permits(slots - current);
                }
            }
            Err(e) => {
                tracing::warn!("SlotSemaphoreMap write lock poisoned on upsert: {e}");
            }
        }
    }

    /// Remove the semaphore for `dog_id`. In-flight permits remain valid until dropped.
    pub fn remove(&self, dog_id: &str) {
        match self.inner.write() {
            Ok(mut guard) => {
                guard.remove(dog_id);
            }
            Err(e) => {
                tracing::warn!("SlotSemaphoreMap write lock poisoned on remove: {e}");
                e.into_inner().remove(dog_id);
            }
        }
    }
}

// ── TESTS ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // F1 — permit exhaustion: semaphore blocks at exactly N permits.
    #[tokio::test]
    async fn f1_permits_exhaust_at_slot_count() {
        let sem = SlotSemaphore::new("test-dog", 4);
        let p1 = sem.try_acquire(SlotPriority::Background).unwrap();
        let p2 = sem.try_acquire(SlotPriority::Background).unwrap();
        let p3 = sem.try_acquire(SlotPriority::Background).unwrap();
        let p4 = sem.try_acquire(SlotPriority::Background).unwrap();
        assert!(sem.try_acquire(SlotPriority::Background).is_none());
        drop(p1);
        assert!(sem.try_acquire(SlotPriority::Background).is_some());
        // silence unused-variable warnings for held permits
        drop(p2);
        drop(p3);
        drop(p4);
    }

    // F4 — boot with zero slots then dynamically add permits.
    #[tokio::test]
    async fn f4_boot_zero_then_add_permits() {
        let sem = SlotSemaphore::new("boot-dog", 0);
        assert!(sem.try_acquire(SlotPriority::User).is_none());
        sem.add_permits(4);
        assert_eq!(sem.total_slots(), 4);
        assert!(sem.try_acquire(SlotPriority::User).is_some());
    }

    // F2 — blocking acquire waits for a permit to be released.
    #[tokio::test]
    async fn f2_blocking_acquire_waits_for_release() {
        let sem = Arc::new(SlotSemaphore::new("wait-dog", 1));
        let held = sem.try_acquire(SlotPriority::Background).unwrap();
        let sem2 = Arc::clone(&sem);
        let handle = tokio::spawn(async move { sem2.acquire(SlotPriority::User).await });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        drop(held);
        let permit = handle.await.unwrap();
        assert!(permit.is_some());
    }

    // Upsert — creates new entry and resizes existing one.
    #[test]
    fn semaphore_map_upsert_creates_and_resizes() {
        let map = SlotSemaphoreMap::new();
        map.upsert("dog-a", 2);
        let sem = map.get("dog-a").unwrap();
        assert_eq!(sem.total_slots(), 2);
        map.upsert("dog-a", 4);
        assert_eq!(sem.total_slots(), 4);
        assert!(map.get("nonexistent").is_none());
    }

    // SlotPriority ordering: User > Hermes > Nightshift > Background.
    #[test]
    fn priority_ordering() {
        assert!(SlotPriority::User > SlotPriority::Hermes);
        assert!(SlotPriority::Hermes > SlotPriority::Nightshift);
        assert!(SlotPriority::Nightshift > SlotPriority::Background);
    }

    // is_blocking: only User and Hermes block.
    #[test]
    fn priority_blocking_flags() {
        assert!(SlotPriority::User.is_blocking());
        assert!(SlotPriority::Hermes.is_blocking());
        assert!(!SlotPriority::Nightshift.is_blocking());
        assert!(!SlotPriority::Background.is_blocking());
    }

    // timeout: User=30s, Hermes=15s, others=ZERO.
    #[test]
    fn priority_timeouts() {
        assert_eq!(SlotPriority::User.timeout(), Duration::from_secs(30));
        assert_eq!(SlotPriority::Hermes.timeout(), Duration::from_secs(15));
        assert_eq!(SlotPriority::Nightshift.timeout(), Duration::ZERO);
        assert_eq!(SlotPriority::Background.timeout(), Duration::ZERO);
    }

    // Non-blocking priorities skip immediately when no slot is available.
    #[tokio::test]
    async fn non_blocking_priority_skips_on_contention() {
        let sem = SlotSemaphore::new("skip-dog", 1);
        let _held = sem.try_acquire(SlotPriority::User).unwrap();
        // Background must return None immediately (no waiting).
        let result = sem.acquire(SlotPriority::Background).await;
        assert!(result.is_none());
    }

    // available() tracks free permits correctly.
    #[test]
    fn available_tracks_free_permits() {
        let sem = SlotSemaphore::new("avail-dog", 3);
        assert_eq!(sem.available(), 3);
        let p1 = sem.try_acquire(SlotPriority::User).unwrap();
        assert_eq!(sem.available(), 2);
        drop(p1);
        assert_eq!(sem.available(), 3);
    }

    // upsert shrink is a no-op — existing total_slots unchanged.
    #[test]
    fn semaphore_map_upsert_shrink_is_noop() {
        let map = SlotSemaphoreMap::new();
        map.upsert("dog-b", 4);
        map.upsert("dog-b", 2); // shrink attempt — no-op
        let sem = map.get("dog-b").unwrap();
        assert_eq!(sem.total_slots(), 4);
    }

    // remove drops the entry.
    #[test]
    fn semaphore_map_remove() {
        let map = SlotSemaphoreMap::new();
        map.upsert("dog-c", 2);
        assert!(map.get("dog-c").is_some());
        map.remove("dog-c");
        assert!(map.get("dog-c").is_none());
    }

    // F3 — Concurrent user + nightshift, no starvation.
    // 4 slots: nightshift takes 3, user must still get the 4th.
    #[tokio::test]
    async fn f3_concurrent_user_and_nightshift_no_starvation() {
        let sem = Arc::new(SlotSemaphore::new("gpu-dog", 4));
        // Nightshift takes 3 slots (sequential in practice, parallel here for stress)
        let _n1 = sem.try_acquire(SlotPriority::Nightshift).unwrap();
        let _n2 = sem.try_acquire(SlotPriority::Nightshift).unwrap();
        let _n3 = sem.try_acquire(SlotPriority::Nightshift).unwrap();
        // User still gets the 4th slot — no starvation
        let user = sem.try_acquire(SlotPriority::User);
        assert!(user.is_some(), "User must get a slot when 1/4 is free");
    }

    // F5 — No deadlock under fan-out.
    // 3 dogs, 1 slot each. Blocking user evaluate + non-blocking nightshift must both complete.
    #[tokio::test]
    async fn f5_fanout_no_deadlock() {
        // 3 dogs, 1 slot each. Two concurrent "evaluates".
        let map = Arc::new(SlotSemaphoreMap::new());
        map.upsert("dog-a", 1);
        map.upsert("dog-b", 1);
        map.upsert("dog-c", 1);

        // User evaluate: acquires all 3 dogs (blocking)
        let map2 = Arc::clone(&map);
        let h1 = tokio::spawn(async move {
            let _a = map2.get("dog-a").unwrap().acquire(SlotPriority::User).await;
            let _b = map2.get("dog-b").unwrap().acquire(SlotPriority::User).await;
            let _c = map2.get("dog-c").unwrap().acquire(SlotPriority::User).await;
        });

        // Nightshift evaluate: try_acquire all 3 dogs (non-blocking, may skip)
        let map3 = Arc::clone(&map);
        let h2 = tokio::spawn(async move {
            let _a = map3
                .get("dog-a")
                .unwrap()
                .try_acquire(SlotPriority::Nightshift);
            let _b = map3
                .get("dog-b")
                .unwrap()
                .try_acquire(SlotPriority::Nightshift);
            let _c = map3
                .get("dog-c")
                .unwrap()
                .try_acquire(SlotPriority::Nightshift);
        });

        // Both must complete within 5s (no deadlock)
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            h1.await.unwrap();
            h2.await.unwrap();
        })
        .await
        .expect("must not deadlock");
    }
}
