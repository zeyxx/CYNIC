//! Bounded Ingestion Buffer and Spool.
//!
//! Enforces the Obs1 bounded ingestion invariant: observation pipelines must
//! bound their in-memory buffers and measure every drop or eviction with
//! atomic counters (no uncounted drops / amnesia).
//!
//! Adapts to metabolic pressure: supports `DropOldest`, `RejectNew`, and batch draining.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Policy applied when the buffer reaches its maximum capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverflowPolicy {
    /// Evict the oldest item in the buffer to make room for the new incoming item.
    DropOldest,
    /// Reject the incoming item, returning `BufferError::CapacityExceeded`.
    RejectNew,
}

impl Default for OverflowPolicy {
    fn default() -> Self {
        Self::DropOldest
    }
}

/// Point-in-time metrics snapshot of the ingestion buffer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BufferMetricsSnapshot {
    pub capacity: usize,
    pub current_len: usize,
    pub ingested_total: u64,
    pub dropped_total: u64,
    pub drained_total: u64,
    pub drop_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BufferError {
    CapacityExceeded(usize),
    LockPoisoned,
}

impl std::fmt::Display for BufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapacityExceeded(cap) => write!(f, "buffer capacity exceeded ({cap} items)"),
            Self::LockPoisoned => write!(f, "buffer lock poisoned"),
        }
    }
}

impl std::error::Error for BufferError {}

/// Thread-safe, bounded in-memory buffer with atomic drop accounting.
#[derive(Debug)]
pub struct BoundedIngestionBuffer<T> {
    queue: Mutex<VecDeque<T>>,
    capacity: usize,
    policy: OverflowPolicy,
    ingested_total: AtomicU64,
    dropped_total: AtomicU64,
    drained_total: AtomicU64,
}

impl<T> BoundedIngestionBuffer<T> {
    /// Create a new bounded buffer with given capacity and overflow policy.
    pub fn new(capacity: usize, policy: OverflowPolicy) -> Self {
        assert!(capacity > 0, "buffer capacity must be strictly positive");
        Self {
            queue: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
            policy,
            ingested_total: AtomicU64::new(0),
            dropped_total: AtomicU64::new(0),
            drained_total: AtomicU64::new(0),
        }
    }

    /// Push an item into the buffer according to the configured overflow policy.
    pub fn push(&self, item: T) -> Result<(), BufferError> {
        let mut queue = self.queue.lock().map_err(|_| BufferError::LockPoisoned)?;

        self.ingested_total.fetch_add(1, Ordering::Relaxed);

        if queue.len() >= self.capacity {
            match self.policy {
                OverflowPolicy::DropOldest => {
                    let _evicted = queue.pop_front();
                    self.dropped_total.fetch_add(1, Ordering::Relaxed);
                    queue.push_back(item);
                    Ok(())
                }
                OverflowPolicy::RejectNew => {
                    self.dropped_total.fetch_add(1, Ordering::Relaxed);
                    Err(BufferError::CapacityExceeded(self.capacity))
                }
            }
        } else {
            queue.push_back(item);
            Ok(())
        }
    }

    /// Push a batch of items. Returns the count of successfully queued items.
    pub fn push_batch(&self, items: Vec<T>) -> usize {
        let mut accepted = 0;
        for item in items {
            if self.push(item).is_ok() {
                accepted += 1;
            }
        }
        accepted
    }

    /// Drain up to `max_items` from the front of the buffer.
    pub fn drain(&self, max_items: usize) -> Vec<T> {
        let mut queue = match self.queue.lock() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };

        let count = max_items.min(queue.len());
        let mut drained = Vec::with_capacity(count);

        for _ in 0..count {
            if let Some(item) = queue.pop_front() {
                drained.push(item);
            }
        }

        self.drained_total
            .fetch_add(drained.len() as u64, Ordering::Relaxed);
        drained
    }

    /// Drain all currently buffered items.
    pub fn drain_all(&self) -> Vec<T> {
        let mut queue = match self.queue.lock() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };

        let count = queue.len();
        let mut drained = Vec::with_capacity(count);

        while let Some(item) = queue.pop_front() {
            drained.push(item);
        }

        self.drained_total
            .fetch_add(drained.len() as u64, Ordering::Relaxed);
        drained
    }

    /// Get current buffer length.
    pub fn len(&self) -> usize {
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
    }

    /// True if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Maximum capacity of this buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Snapshot current metrics and drop ratios.
    pub fn metrics(&self) -> BufferMetricsSnapshot {
        let ingested = self.ingested_total.load(Ordering::Relaxed);
        let dropped = self.dropped_total.load(Ordering::Relaxed);
        let drained = self.drained_total.load(Ordering::Relaxed);
        let current_len = self.len();

        let total_attempts = ingested;
        let drop_rate = if total_attempts > 0 {
            dropped as f64 / total_attempts as f64
        } else {
            0.0
        };

        BufferMetricsSnapshot {
            capacity: self.capacity,
            current_len,
            ingested_total: ingested,
            dropped_total: dropped,
            drained_total: drained,
            drop_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_respects_capacity_with_drop_oldest() {
        let buffer = BoundedIngestionBuffer::new(3, OverflowPolicy::DropOldest);

        assert!(buffer.push(1).is_ok());
        assert!(buffer.push(2).is_ok());
        assert!(buffer.push(3).is_ok());
        assert_eq!(buffer.len(), 3);

        // 4th push evicts 1
        assert!(buffer.push(4).is_ok());
        assert_eq!(buffer.len(), 3);

        let items = buffer.drain_all();
        assert_eq!(items, vec![2, 3, 4]);

        let m = buffer.metrics();
        assert_eq!(m.ingested_total, 4);
        assert_eq!(m.dropped_total, 1);
        assert_eq!(m.drained_total, 3);
        assert_eq!(m.drop_rate, 0.25);
    }

    #[test]
    fn buffer_respects_capacity_with_reject_new() {
        let buffer = BoundedIngestionBuffer::new(2, OverflowPolicy::RejectNew);

        assert!(buffer.push("a").is_ok());
        assert!(buffer.push("b").is_ok());
        assert_eq!(buffer.len(), 2);

        // 3rd push fails
        let err = buffer.push("c");
        assert_eq!(err, Err(BufferError::CapacityExceeded(2)));

        let items = buffer.drain_all();
        assert_eq!(items, vec!["a", "b"]);

        let m = buffer.metrics();
        assert_eq!(m.ingested_total, 3);
        assert_eq!(m.dropped_total, 1);
        assert_eq!(m.drained_total, 2);
    }

    #[test]
    fn partial_drain_preserves_order() {
        let buffer = BoundedIngestionBuffer::new(10, OverflowPolicy::DropOldest);
        buffer.push_batch(vec![10, 20, 30, 40, 50]);

        let first_two = buffer.drain(2);
        assert_eq!(first_two, vec![10, 20]);
        assert_eq!(buffer.len(), 3);

        let remaining = buffer.drain_all();
        assert_eq!(remaining, vec![30, 40, 50]);
        assert_eq!(buffer.len(), 0);
    }
}
