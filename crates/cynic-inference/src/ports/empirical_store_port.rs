//! Empirical Performance Store Port.
//!
//! Abstract contract for recording, querying, and solving optimal model execution profiles
//! from empirical telemetry.

use async_trait::async_trait;
use cynic_protocol::ContextTier;
use thiserror::Error;

use crate::domain::matrix_sweep::HardwareBudget;
use crate::domain::matrix_sweep::empirical_store::{
    ConstraintFilter, EmpiricalProfileEntry, ModelEmpiricalScorecard, OptimizationGoal,
};

#[derive(Debug, Error)]
pub enum EmpiricalStoreError {
    #[error("I/O error accessing empirical ledger at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Serialization / deserialization error in empirical store: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Model profile not found in empirical ledger for '{model_alias}'")]
    ModelNotFound { model_alias: String },

    #[error("Corrupted record in empirical ledger: {details}")]
    CorruptedRecord { details: String },
}

/// Abstract port for querying and updating empirical model profiles.
#[async_trait]
pub trait EmpiricalStorePort: Send + Sync {
    /// Append a single verified empirical benchmark result to the durable store.
    async fn record_entry(&self, entry: EmpiricalProfileEntry) -> Result<(), EmpiricalStoreError>;

    /// Append a batch of verified empirical benchmark results atomically.
    async fn record_batch(
        &self,
        entries: Vec<EmpiricalProfileEntry>,
    ) -> Result<(), EmpiricalStoreError>;

    /// Query raw empirical entries matching the given criteria.
    async fn query_entries(
        &self,
        model_alias: Option<&str>,
        context_tier: Option<ContextTier>,
    ) -> Result<Vec<EmpiricalProfileEntry>, EmpiricalStoreError>;

    /// Pure constraint solving: finds the proven best configuration for a model.
    async fn resolve_optimal(
        &self,
        model_alias: &str,
        live_hardware: HardwareBudget,
        system_reserve_bytes: u64,
        constraints: &ConstraintFilter,
        goal: OptimizationGoal,
    ) -> Result<Option<EmpiricalProfileEntry>, EmpiricalStoreError>;

    /// Get the aggregated performance scorecard for a model.
    async fn get_scorecard(
        &self,
        model_alias: &str,
    ) -> Result<ModelEmpiricalScorecard, EmpiricalStoreError>;
}
