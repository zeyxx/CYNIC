//! Priority queue for φ-weighted transaction scheduling

use crate::{ReputationScore, Result, SchedulerError, Verdict, PHI, PHI_INV};
use parking_lot::Mutex;
use priority_queue::PriorityQueue as InnerPriorityQueue;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Transaction priority score
#[derive(Debug, Clone, Copy)]
pub struct TransactionPriority {
    /// Base priority fee (lamports per compute unit)
    pub base_fee: u64,
    /// φ-weighted priority score
    pub phi_score: f64,
    /// Timestamp for FIFO within same priority
    pub timestamp: Instant,
}

impl TransactionPriority {
    /// Create new priority from fee and reputation
    pub fn new(base_fee: u64, reputation: &ReputationScore) -> Self {
        let multiplier = reputation.verdict.multiplier();
        let phi_score = (base_fee as f64) * multiplier;

        Self {
            base_fee,
            phi_score,
            timestamp: Instant::now(),
        }
    }

    /// Create priority for unknown reputation (neutral)
    pub fn neutral(base_fee: u64) -> Self {
        Self {
            base_fee,
            phi_score: base_fee as f64,
            timestamp: Instant::now(),
        }
    }

    /// Create boosted priority (WAG)
    pub fn boosted(base_fee: u64) -> Self {
        Self {
            base_fee,
            phi_score: (base_fee as f64) * PHI,
            timestamp: Instant::now(),
        }
    }

    /// Create reduced priority (BARK)
    pub fn reduced(base_fee: u64) -> Self {
        Self {
            base_fee,
            phi_score: (base_fee as f64) * PHI_INV,
            timestamp: Instant::now(),
        }
    }

    /// Create zero priority (GROWL - should be dropped)
    pub fn zero() -> Self {
        Self {
            base_fee: 0,
            phi_score: 0.0,
            timestamp: Instant::now(),
        }
    }
}

impl PartialEq for TransactionPriority {
    fn eq(&self, other: &Self) -> bool {
        self.phi_score == other.phi_score && self.timestamp == other.timestamp
    }
}

impl Eq for TransactionPriority {}

impl PartialOrd for TransactionPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TransactionPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher phi_score = higher priority
        match self.phi_score.partial_cmp(&other.phi_score) {
            Some(Ordering::Equal) | None => {
                // Earlier timestamp = higher priority (FIFO for same score)
                other.timestamp.cmp(&self.timestamp)
            }
            Some(ord) => ord,
        }
    }
}

/// Transaction entry in the queue
#[derive(Debug, Clone)]
pub struct QueuedTransaction {
    /// Transaction signature (for dedup)
    pub signature: String,
    /// Fee payer address
    pub fee_payer: String,
    /// Priority fee in lamports per CU
    pub priority_fee: u64,
    /// Compute units requested
    pub compute_units: u64,
    /// Reputation score (if fetched)
    pub reputation: Option<ReputationScore>,
    /// Raw transaction bytes offset (for shared memory)
    pub tx_offset: usize,
    /// Raw transaction bytes length
    pub tx_length: u32,
}

/// Thread-safe priority queue for transactions
pub struct PriorityQueue {
    inner: Arc<Mutex<PriorityQueueInner>>,
    max_size: usize,
}

struct PriorityQueueInner {
    queue: InnerPriorityQueue<String, TransactionPriority>,
    transactions: HashMap<String, QueuedTransaction>,
    stats: QueueStats,
}

/// Queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    /// Total transactions enqueued
    pub total_enqueued: u64,
    /// Total transactions dequeued
    pub total_dequeued: u64,
    /// Total transactions dropped (GROWL)
    pub total_dropped: u64,
    /// Total transactions boosted (WAG)
    pub total_boosted: u64,
    /// Total transactions reduced (BARK)
    pub total_reduced: u64,
    /// Current queue size
    pub current_size: usize,
}

impl PriorityQueue {
    /// Create a new priority queue
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PriorityQueueInner {
                queue: priority_queue::PriorityQueue::new(),
                transactions: HashMap::new(),
                stats: QueueStats::default(),
            })),
            max_size,
        }
    }

    /// Enqueue a transaction with reputation-based priority
    pub fn enqueue(&self, tx: QueuedTransaction, reputation: &ReputationScore) -> Result<bool> {
        let mut inner = self.inner.lock();

        // Check if should drop (GROWL)
        if reputation.verdict.should_drop() {
            inner.stats.total_dropped += 1;
            return Ok(false);
        }

        // Check queue capacity
        if inner.queue.len() >= self.max_size {
            // Drop lowest priority if new tx has higher priority
            let priority = TransactionPriority::new(tx.priority_fee, reputation);
            if let Some((_, lowest)) = inner.queue.peek() {
                if priority <= *lowest {
                    return Err(SchedulerError::queue("Queue full, transaction priority too low"));
                }
                // Remove lowest priority
                inner.queue.pop();
            }
        }

        // Calculate priority
        let priority = TransactionPriority::new(tx.priority_fee, reputation);

        // Track boost/reduce
        match reputation.verdict {
            Verdict::Wag => inner.stats.total_boosted += 1,
            Verdict::Bark => inner.stats.total_reduced += 1,
            _ => {}
        }

        // Insert
        let sig = tx.signature.clone();
        inner.transactions.insert(sig.clone(), tx);
        inner.queue.push(sig, priority);
        inner.stats.total_enqueued += 1;
        inner.stats.current_size = inner.queue.len();

        Ok(true)
    }

    /// Dequeue highest priority transaction
    pub fn dequeue(&self) -> Option<QueuedTransaction> {
        let mut inner = self.inner.lock();

        if let Some((sig, _)) = inner.queue.pop() {
            inner.stats.total_dequeued += 1;
            inner.stats.current_size = inner.queue.len();
            return inner.transactions.remove(&sig);
        }

        None
    }

    /// Dequeue up to `n` highest priority transactions
    pub fn dequeue_batch(&self, n: usize) -> Vec<QueuedTransaction> {
        let mut inner = self.inner.lock();
        let mut batch = Vec::with_capacity(n);

        for _ in 0..n {
            if let Some((sig, _)) = inner.queue.pop() {
                if let Some(tx) = inner.transactions.remove(&sig) {
                    batch.push(tx);
                }
            } else {
                break;
            }
        }

        inner.stats.total_dequeued += batch.len() as u64;
        inner.stats.current_size = inner.queue.len();

        batch
    }

    /// Get current queue size
    pub fn len(&self) -> usize {
        self.inner.lock().queue.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.inner.lock().queue.is_empty()
    }

    /// Get queue statistics
    pub fn stats(&self) -> QueueStats {
        self.inner.lock().stats.clone()
    }

    /// Clear the queue
    pub fn clear(&self) {
        let mut inner = self.inner.lock();
        inner.queue.clear();
        inner.transactions.clear();
        inner.stats.current_size = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tx(sig: &str, fee: u64) -> QueuedTransaction {
        QueuedTransaction {
            signature: sig.to_string(),
            fee_payer: "test_payer".to_string(),
            priority_fee: fee,
            compute_units: 200_000,
            reputation: None,
            tx_offset: 0,
            tx_length: 100,
        }
    }

    #[test]
    fn test_priority_ordering() {
        let p1 = TransactionPriority::neutral(100);
        let p2 = TransactionPriority::boosted(100);
        let p3 = TransactionPriority::reduced(100);

        assert!(p2 > p1);
        assert!(p1 > p3);
        assert!(p2 > p3);
    }

    #[test]
    fn test_queue_basic() {
        let queue = PriorityQueue::new(100);

        let tx = make_tx("sig1", 1000);
        let rep = ReputationScore::default();

        assert!(queue.enqueue(tx, &rep).unwrap());
        assert_eq!(queue.len(), 1);

        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued.signature, "sig1");
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_priority_order() {
        let queue = PriorityQueue::new(100);

        // Add low priority
        let tx1 = make_tx("low", 100);
        let rep1 = ReputationScore {
            verdict: Verdict::Bark,
            ..Default::default()
        };
        queue.enqueue(tx1, &rep1).unwrap();

        // Add high priority
        let tx2 = make_tx("high", 100);
        let rep2 = ReputationScore {
            verdict: Verdict::Wag,
            ..Default::default()
        };
        queue.enqueue(tx2, &rep2).unwrap();

        // Add neutral
        let tx3 = make_tx("neutral", 100);
        let rep3 = ReputationScore::default();
        queue.enqueue(tx3, &rep3).unwrap();

        // Should dequeue in order: high > neutral > low
        assert_eq!(queue.dequeue().unwrap().signature, "high");
        assert_eq!(queue.dequeue().unwrap().signature, "neutral");
        assert_eq!(queue.dequeue().unwrap().signature, "low");
    }

    #[test]
    fn test_growl_filter() {
        let queue = PriorityQueue::new(100);

        let tx = make_tx("growl_tx", 10000);
        let rep = ReputationScore {
            verdict: Verdict::Growl,
            ..Default::default()
        };

        // Should return false (dropped)
        assert!(!queue.enqueue(tx, &rep).unwrap());
        assert!(queue.is_empty());

        let stats = queue.stats();
        assert_eq!(stats.total_dropped, 1);
    }

    #[test]
    fn test_batch_dequeue() {
        let queue = PriorityQueue::new(100);
        let rep = ReputationScore::default();

        for i in 0..10 {
            let tx = make_tx(&format!("tx{}", i), 100);
            queue.enqueue(tx, &rep).unwrap();
        }

        let batch = queue.dequeue_batch(5);
        assert_eq!(batch.len(), 5);
        assert_eq!(queue.len(), 5);
    }
}
