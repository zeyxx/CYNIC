//! Driven ports for personal anchoring.
//!
//! These traits define the interface between the pure domain logic (in
//! `cynic-anchoring::domain`) and the storage/sync adapters. The runner
//! injects concrete implementations.
//!
//! Authority: foundation.json §37 (portable_host_contract).

use cynic_protocol::observation_record::Observation;

use crate::domain::AnchorOutcome;
use crate::domain::GeneralReturn;

/// Port for storing and retrieving anchor observations.
pub trait AnchorEventStore {
    /// Record a new anchor observation. Returns the assigned id.
    fn record(&self, obs: &Observation) -> Result<u64, String>;

    /// Retrieve recent anchor observations, newest first.
    fn recent(&self, limit: usize) -> Result<Vec<Observation>, String>;

    /// Retrieve observations pending sync to the kernel.
    fn pending_sync(&self, limit: usize) -> Result<Vec<Observation>, String>;

    /// Mark observations as synced.
    fn mark_synced(&self, ids: &[u64], synced_at_ms: i64) -> Result<(), String>;
}

/// Port for projecting anchor observations into a general return view.
pub trait AnchorProjection {
    /// Compute the general return at instant T for a given time window.
    fn general_return(
        &self,
        window_start_ms: i64,
        window_end_ms: i64,
    ) -> Result<GeneralReturn, String>;
}

/// Port for syncing anchor observations to the CYNIC kernel.
pub trait AnchorSyncPort {
    /// Sync a batch of anchor observations to the kernel. Returns count synced.
    fn sync_batch(&self, observations: &[Observation]) -> Result<u32, String>;
}

/// Port for storing and retrieving anchor outcomes.
pub trait AnchorOutcomeStore {
    /// Record a decision→result outcome.
    fn record_outcome(&self, outcome: &AnchorOutcome) -> Result<(), String>;

    /// Retrieve recent outcomes.
    fn recent_outcomes(&self, limit: usize) -> Result<Vec<AnchorOutcome>, String>;
}