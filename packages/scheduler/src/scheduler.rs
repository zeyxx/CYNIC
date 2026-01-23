//! Main CYNIC Scheduler implementation

use crate::{
    config::SchedulerConfig,
    cynic_client::CynicClient,
    error::{Result, SchedulerError},
    priority::{PriorityQueue, QueuedTransaction, QueueStats},
    ReputationScore,
};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Scheduler state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerState {
    /// Not yet started
    Stopped,
    /// Starting up
    Starting,
    /// Running and processing transactions
    Running,
    /// Shutting down
    Stopping,
}

/// CYNIC Scheduler statistics
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    /// Queue statistics
    pub queue: QueueStats,
    /// Total transactions received from TPU
    pub tpu_received: u64,
    /// Total transactions sent to workers
    pub worker_sent: u64,
    /// Total execution results received
    pub results_received: u64,
    /// Total successful executions
    pub successful: u64,
    /// Total failed executions
    pub failed: u64,
    /// Current slot
    pub current_slot: u64,
    /// Is leader
    pub is_leader: bool,
    /// API calls to CYNIC
    pub cynic_api_calls: u64,
    /// API cache hits
    pub cynic_cache_hits: u64,
}

/// CYNIC Scheduler - φ-weighted transaction scheduling for Solana
#[derive(Clone)]
pub struct CynicScheduler {
    config: SchedulerConfig,
    cynic_client: Arc<CynicClient>,
    priority_queue: Arc<PriorityQueue>,
    state: Arc<RwLock<SchedulerState>>,
    running: Arc<AtomicBool>,
    current_slot: Arc<AtomicU64>,
    is_leader: Arc<AtomicBool>,
    stats: Arc<RwLock<SchedulerStats>>,
}

impl CynicScheduler {
    /// Create a new CYNIC scheduler
    pub fn new(config: SchedulerConfig) -> Result<Self> {
        config.validate()?;

        let cynic_client = Arc::new(CynicClient::new(
            &config.cynic_url,
            config.cynic_api_key.clone(),
            config.reputation_cache_ttl,
            config.api_timeout,
        )?);

        let priority_queue = Arc::new(PriorityQueue::new(config.max_queue_size));

        Ok(Self {
            config,
            cynic_client,
            priority_queue,
            state: Arc::new(RwLock::new(SchedulerState::Stopped)),
            running: Arc::new(AtomicBool::new(false)),
            current_slot: Arc::new(AtomicU64::new(0)),
            is_leader: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(SchedulerStats::default())),
        })
    }

    /// Start the scheduler
    pub async fn start(&self) -> Result<()> {
        {
            let mut state = self.state.write();
            if *state != SchedulerState::Stopped {
                return Err(SchedulerError::internal("Scheduler already running"));
            }
            *state = SchedulerState::Starting;
        }

        info!(
            "Starting CYNIC Scheduler with {} workers",
            self.config.num_workers
        );

        self.running.store(true, Ordering::SeqCst);

        // TODO: Initialize shared memory connections
        // - Connect to tpu_to_pack queue
        // - Connect to progress_tracker queue
        // - Set up worker queues

        {
            let mut state = self.state.write();
            *state = SchedulerState::Running;
        }

        info!("CYNIC Scheduler started");
        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&self) -> Result<()> {
        {
            let mut state = self.state.write();
            if *state != SchedulerState::Running {
                return Ok(());
            }
            *state = SchedulerState::Stopping;
        }

        info!("Stopping CYNIC Scheduler");

        self.running.store(false, Ordering::SeqCst);

        // TODO: Cleanup shared memory connections

        {
            let mut state = self.state.write();
            *state = SchedulerState::Stopped;
        }

        info!("CYNIC Scheduler stopped");
        Ok(())
    }

    /// Process an incoming transaction from TPU
    pub async fn process_transaction(
        &self,
        signature: String,
        fee_payer: String,
        priority_fee: u64,
        compute_units: u64,
        tx_offset: usize,
        tx_length: u32,
    ) -> Result<bool> {
        // Update stats
        {
            let mut stats = self.stats.write();
            stats.tpu_received += 1;
        }

        // Get reputation for fee payer
        let reputation = self.get_reputation(&fee_payer).await;

        // Check GROWL filter
        if self.config.enable_growl_filter && reputation.verdict.should_drop() {
            debug!(
                signature = %signature,
                fee_payer = %fee_payer,
                verdict = ?reputation.verdict,
                "Dropping GROWL transaction"
            );
            return Ok(false);
        }

        // Check minimum E-Score
        if let Some(e_score) = reputation.e_score {
            if e_score < self.config.min_e_score {
                debug!(
                    signature = %signature,
                    fee_payer = %fee_payer,
                    e_score = %e_score,
                    min_e_score = %self.config.min_e_score,
                    "Dropping low E-Score transaction"
                );
                return Ok(false);
            }
        }

        // Create queued transaction
        let tx = QueuedTransaction {
            signature,
            fee_payer,
            priority_fee,
            compute_units,
            reputation: Some(reputation.clone()),
            tx_offset,
            tx_length,
        };

        // Enqueue with φ-weighted priority
        self.priority_queue.enqueue(tx, &reputation)
    }

    /// Get reputation for an address (with caching)
    async fn get_reputation(&self, address: &str) -> ReputationScore {
        match self.cynic_client.get_wallet_reputation(address).await {
            Ok(score) => {
                let mut stats = self.stats.write();
                stats.cynic_api_calls += 1;
                score
            }
            Err(e) => {
                warn!(
                    address = %address,
                    error = %e,
                    "Failed to get reputation, using default"
                );
                ReputationScore::default()
            }
        }
    }

    /// Get batch of transactions for block packing
    pub fn get_batch(&self, max_size: usize) -> Vec<QueuedTransaction> {
        let batch = self.priority_queue.dequeue_batch(max_size);

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.worker_sent += batch.len() as u64;
        }

        batch
    }

    /// Update slot/leader status from progress tracker
    pub fn update_progress(&self, slot: u64, is_leader: bool) {
        self.current_slot.store(slot, Ordering::SeqCst);
        self.is_leader.store(is_leader, Ordering::SeqCst);

        let mut stats = self.stats.write();
        stats.current_slot = slot;
        stats.is_leader = is_leader;
    }

    /// Record execution result
    pub fn record_result(&self, success: bool) {
        let mut stats = self.stats.write();
        stats.results_received += 1;
        if success {
            stats.successful += 1;
        } else {
            stats.failed += 1;
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> SchedulerStats {
        let mut stats = self.stats.read().clone();
        stats.queue = self.priority_queue.stats();

        // Get cache stats
        let (valid, _total) = self.cynic_client.cache_stats();
        stats.cynic_cache_hits = valid as u64;

        stats
    }

    /// Get current state
    pub fn state(&self) -> SchedulerState {
        *self.state.read()
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get current slot
    pub fn current_slot(&self) -> u64 {
        self.current_slot.load(Ordering::SeqCst)
    }

    /// Check if currently leader
    pub fn is_leader(&self) -> bool {
        self.is_leader.load(Ordering::SeqCst)
    }

    /// Get queue length
    pub fn queue_len(&self) -> usize {
        self.priority_queue.len()
    }
}

/// Display stats in a readable format
impl std::fmt::Display for SchedulerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CYNIC Scheduler Stats:\n\
             ├─ Slot: {} (Leader: {})\n\
             ├─ Queue: {} txs\n\
             ├─ TPU → Queue: {} received\n\
             ├─ Queue → Workers: {} sent\n\
             ├─ Results: {} ({} ok, {} fail)\n\
             ├─ Dropped (GROWL): {}\n\
             ├─ Boosted (WAG): {}\n\
             ├─ Reduced (BARK): {}\n\
             └─ CYNIC API: {} calls, {} cache hits",
            self.current_slot,
            if self.is_leader { "yes" } else { "no" },
            self.queue.current_size,
            self.tpu_received,
            self.worker_sent,
            self.results_received,
            self.successful,
            self.failed,
            self.queue.total_dropped,
            self.queue.total_boosted,
            self.queue.total_reduced,
            self.cynic_api_calls,
            self.cynic_cache_hits,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = SchedulerConfig::default();
        let scheduler = CynicScheduler::new(config);
        assert!(scheduler.is_ok());
    }

    #[tokio::test]
    async fn test_scheduler_start_stop() {
        let config = SchedulerConfig::default();
        let scheduler = CynicScheduler::new(config).unwrap();

        assert_eq!(scheduler.state(), SchedulerState::Stopped);

        scheduler.start().await.unwrap();
        assert_eq!(scheduler.state(), SchedulerState::Running);

        scheduler.stop().await.unwrap();
        assert_eq!(scheduler.state(), SchedulerState::Stopped);
    }
}
