//! InferenceQueue — per-Dog priority queue for inference slot access.
//!
//! Replaces the binary pass/skip semaphore with a bounded priority queue.
//! When the slot is busy, requests wait in priority order (User > Hermes > Nightshift > Background).
//! When the slot frees, the highest-priority waiter is woken — not FIFO, not random.
//!
//! Bounded: queue depth is capped. Beyond capacity, lowest-priority requests are rejected.
//! This prevents unbounded memory growth from flood callers (K9: bounded everything).

use std::cmp::Ordering as CmpOrd;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::oneshot;

use super::slot_semaphore::SlotPriority;

// ── QueuedRequest ────────────────────────────────────────────

struct QueuedRequest {
    priority: SlotPriority,
    enqueued_at: Instant,
    wake_tx: oneshot::Sender<()>,
}

// Higher priority first; FIFO within same priority (earlier enqueue = dequeue first).
impl Ord for QueuedRequest {
    fn cmp(&self, other: &Self) -> CmpOrd {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.enqueued_at.cmp(&self.enqueued_at))
    }
}

impl PartialOrd for QueuedRequest {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrd> {
        Some(self.cmp(other))
    }
}

impl PartialEq for QueuedRequest {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.enqueued_at == other.enqueued_at
    }
}

impl Eq for QueuedRequest {}

// ── QueuePermit (RAII guard) ─────────────────────────────────

/// Holding this means the caller owns the inference slot.
/// Dropping it releases the slot and wakes the next waiter.
pub struct QueuePermit {
    queue: Arc<InferenceQueue>,
    pub dog_id: String,
    pub priority: SlotPriority,
}

impl Drop for QueuePermit {
    fn drop(&mut self) {
        self.queue.release();
    }
}

impl std::fmt::Debug for QueuePermit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueuePermit")
            .field("dog_id", &self.dog_id)
            .field("priority", &self.priority)
            .finish()
    }
}

// ── InferenceQueue ───────────────────────────────────────────

/// Per-Dog priority queue for inference slot access.
///
/// One slot per queue (matching `--parallel 1`). Callers `acquire()` to
/// enter the queue; the highest-priority waiter is woken when the slot frees.
pub struct InferenceQueue {
    pub dog_id: String,
    /// Priority-ordered waiting list. Protected by std::Mutex (sync — used in Drop).
    waiters: Mutex<BinaryHeap<QueuedRequest>>,
    /// Max pending requests. Beyond this, lowest-priority are rejected.
    capacity: u32,
    /// True = slot is free. Protected by std::Mutex (sync — used in Drop).
    slot_free: Mutex<bool>,
}

impl std::fmt::Debug for InferenceQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InferenceQueue")
            .field("dog_id", &self.dog_id)
            .field("queue_depth", &self.queue_depth())
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl InferenceQueue {
    pub fn new(dog_id: impl Into<String>, capacity: u32) -> Self {
        Self {
            dog_id: dog_id.into(),
            waiters: Mutex::new(BinaryHeap::new()),
            capacity,
            slot_free: Mutex::new(true),
        }
    }

    /// Acquire the inference slot with priority ordering.
    ///
    /// - If slot is free → return permit immediately.
    /// - If busy + queue not full → enqueue and wait (priority-ordered).
    /// - If busy + queue full → return None (backpressure).
    /// - If timeout expires → return None (waiter removed on next release cycle).
    pub async fn acquire(self: &Arc<Self>, priority: SlotPriority) -> Option<QueuePermit> {
        // Fast path: slot is free.
        {
            let mut free = self.slot_free.lock().unwrap_or_else(|e| e.into_inner());
            if *free {
                *free = false;
                return Some(QueuePermit {
                    queue: Arc::clone(self),
                    dog_id: self.dog_id.clone(),
                    priority,
                });
            }
        }

        // Queue capacity check.
        let wake_rx = {
            let mut waiters = self.waiters.lock().unwrap_or_else(|e| e.into_inner());
            if waiters.len() as u32 >= self.capacity {
                tracing::debug!(
                    dog_id = %self.dog_id,
                    priority = ?priority,
                    depth = waiters.len(),
                    "inference queue full — rejecting request"
                );
                return None;
            }
            let (tx, rx) = oneshot::channel();
            waiters.push(QueuedRequest {
                priority,
                enqueued_at: Instant::now(),
                wake_tx: tx,
            });
            rx
        };

        // Wait for slot with priority-specific timeout.
        let timeout = match priority {
            SlotPriority::User => Duration::from_secs(60),
            SlotPriority::Hermes => Duration::from_secs(30),
            SlotPriority::Nightshift => Duration::from_secs(30),
            SlotPriority::Background => Duration::from_secs(15),
        };

        match tokio::time::timeout(timeout, wake_rx).await {
            Ok(Ok(())) => Some(QueuePermit {
                queue: Arc::clone(self),
                dog_id: self.dog_id.clone(),
                priority,
            }),
            _ => {
                // Timed out or channel closed. The QueuedRequest remains in the heap
                // with a dead wake_tx. release() will skip it (send fails) and try next.
                tracing::debug!(
                    dog_id = %self.dog_id,
                    priority = ?priority,
                    "inference queue: waiter timed out"
                );
                None
            }
        }
    }

    /// Release the slot and wake the highest-priority waiter.
    /// Called from QueuePermit::drop (sync context — no async).
    fn release(&self) {
        let mut waiters = self.waiters.lock().unwrap_or_else(|e| e.into_inner());
        // Pop waiters until we find one whose channel is still open.
        while let Some(waiter) = waiters.pop() {
            if waiter.wake_tx.send(()).is_ok() {
                // Woke a waiter — they now hold the slot. Don't set slot_free.
                return;
            }
            // Channel closed (waiter timed out) — skip, try next.
        }
        // No waiters left — mark slot as free.
        let mut free = self.slot_free.lock().unwrap_or_else(|e| e.into_inner());
        *free = true;
    }

    /// Current number of requests waiting in the queue.
    pub fn queue_depth(&self) -> u32 {
        self.waiters.lock().unwrap_or_else(|e| e.into_inner()).len() as u32
    }

    /// Maximum queue capacity.
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Whether the slot is currently free (no active inference).
    pub fn is_idle(&self) -> bool {
        *self.slot_free.lock().unwrap_or_else(|e| e.into_inner())
    }
}

// ── InferenceQueueMap ────────────────────────────────────────

/// Shared registry of per-Dog inference queues.
/// Thread-safe: std::sync::RwLock for sync+async access.
#[derive(Debug, Default)]
pub struct InferenceQueueMap {
    inner: std::sync::RwLock<std::collections::BTreeMap<String, Arc<InferenceQueue>>>,
}

impl InferenceQueueMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the queue for a Dog. Returns None if not registered.
    pub fn get(&self, dog_id: &str) -> Option<Arc<InferenceQueue>> {
        match self.inner.read() {
            Ok(guard) => guard.get(dog_id).cloned(),
            Err(e) => e.into_inner().get(dog_id).cloned(),
        }
    }

    /// Register a queue for a Dog. Idempotent — does not replace existing.
    pub fn upsert(&self, dog_id: &str, capacity: u32) {
        if self.get(dog_id).is_some() {
            return;
        }
        match self.inner.write() {
            Ok(mut guard) => {
                guard
                    .entry(dog_id.to_string())
                    .or_insert_with(|| Arc::new(InferenceQueue::new(dog_id, capacity)));
            }
            Err(e) => {
                tracing::warn!("InferenceQueueMap write lock poisoned on upsert: {e}");
            }
        }
    }

    /// Remove a Dog's queue.
    pub fn remove(&self, dog_id: &str) {
        match self.inner.write() {
            Ok(mut guard) => {
                guard.remove(dog_id);
            }
            Err(e) => {
                e.into_inner().remove(dog_id);
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn immediate_acquire_when_idle() {
        let q = Arc::new(InferenceQueue::new("dog-a", 8));
        let permit = q.acquire(SlotPriority::User).await;
        assert!(permit.is_some());
        assert!(!q.is_idle());
        drop(permit);
        assert!(q.is_idle());
    }

    #[tokio::test]
    async fn priority_ordering_user_before_background() {
        let q = Arc::new(InferenceQueue::new("dog-b", 8));

        // Hold the slot.
        let _held = q.acquire(SlotPriority::Background).await.unwrap();

        // Enqueue: background first, then user.
        let q2 = Arc::clone(&q);
        let bg_handle = tokio::spawn(async move {
            q2.acquire(SlotPriority::Background)
                .await
                .map(|p| p.priority)
        });
        tokio::time::sleep(Duration::from_millis(10)).await;

        let q3 = Arc::clone(&q);
        let user_handle =
            tokio::spawn(async move { q3.acquire(SlotPriority::User).await.map(|p| p.priority) });
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Release the held slot. User should be woken first (higher priority).
        drop(_held);

        let user_result = tokio::time::timeout(Duration::from_secs(2), user_handle)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            user_result,
            Some(SlotPriority::User),
            "User should be woken first"
        );
    }

    #[tokio::test]
    async fn queue_full_rejects() {
        let q = Arc::new(InferenceQueue::new("dog-c", 2));

        // Hold the slot.
        let _held = q.acquire(SlotPriority::User).await.unwrap();

        // Fill the queue (capacity = 2).
        let q2 = Arc::clone(&q);
        let _w1 = tokio::spawn(async move { q2.acquire(SlotPriority::Background).await });
        tokio::time::sleep(Duration::from_millis(10)).await;

        let q3 = Arc::clone(&q);
        let _w2 = tokio::spawn(async move { q3.acquire(SlotPriority::Background).await });
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(q.queue_depth(), 2);

        // Third request should be rejected (queue full).
        let q4 = Arc::clone(&q);
        let rejected = tokio::spawn(async move { q4.acquire(SlotPriority::Background).await });
        let result = tokio::time::timeout(Duration::from_millis(100), rejected)
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_none(), "should be rejected when queue is full");
    }

    #[tokio::test]
    async fn fifo_within_same_priority() {
        let q = Arc::new(InferenceQueue::new("dog-d", 8));
        let _held = q.acquire(SlotPriority::User).await.unwrap();

        // Enqueue two nightshift requests.
        let q2 = Arc::clone(&q);
        let first = tokio::spawn(async move { q2.acquire(SlotPriority::Nightshift).await });
        tokio::time::sleep(Duration::from_millis(10)).await;

        let q3 = Arc::clone(&q);
        let second = tokio::spawn(async move { q3.acquire(SlotPriority::Nightshift).await });
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Release: first enqueued should get the slot first (FIFO within same priority).
        drop(_held);
        let result1 = tokio::time::timeout(Duration::from_secs(1), first)
            .await
            .unwrap()
            .unwrap();
        assert!(result1.is_some(), "first nightshift should get slot");
    }

    #[tokio::test]
    async fn timeout_expires_gracefully() {
        let q = Arc::new(InferenceQueue::new("dog-e", 8));
        let _held = q.acquire(SlotPriority::User).await.unwrap();

        // Enqueue with very short timeout override — we can't override the built-in
        // timeout, but we can test via tokio::time::timeout wrapping the acquire.
        let q2 = Arc::clone(&q);
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            q2.acquire(SlotPriority::Background),
        )
        .await;

        // Should timeout (slot held, 100ms < 15s background timeout).
        assert!(result.is_err(), "should timeout waiting for slot");
    }

    #[test]
    fn queue_map_upsert_idempotent() {
        let map = InferenceQueueMap::new();
        map.upsert("dog-x", 8);
        map.upsert("dog-x", 16); // should not replace
        let q = map.get("dog-x").unwrap();
        assert_eq!(q.capacity(), 8); // original capacity preserved
    }
}
