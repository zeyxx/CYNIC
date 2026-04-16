//! NullStorage — no-op StoragePort adapter for graceful degradation.
//! When the real DB is unavailable, the kernel substitutes this so verdicts
//! still pass through the pipeline (but are not persisted). K14: degraded
//! mode returns explicit errors rather than silently succeeding.

use super::{Observation, ObservationFrequency, SessionTarget, StorageError, StoragePort};
use crate::domain::ccm::Crystal;
use crate::domain::dog::Verdict;
use async_trait::async_trait;

/// No-op storage for graceful degradation when DB is unavailable.
/// Verdicts pass through but are not persisted.
#[derive(Debug)]
pub struct NullStorage;

#[async_trait]
impl StoragePort for NullStorage {
    async fn ping(&self) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "Storage unavailable (DEGRADED mode)".into(),
        ))
    }
    async fn store_verdict(&self, _verdict: &Verdict) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: verdict not persisted (DEGRADED mode)".into(),
        ))
    }
    async fn get_verdict(&self, _id: &str) -> Result<Option<Verdict>, StorageError> {
        Err(StorageError::ConnectionFailed(
            "Storage unavailable (DEGRADED mode)".into(),
        ))
    }
    async fn list_verdicts(&self, _limit: u32) -> Result<Vec<Verdict>, StorageError> {
        Err(StorageError::ConnectionFailed(
            "Storage unavailable (DEGRADED mode)".into(),
        ))
    }
    async fn store_crystal(&self, _crystal: &Crystal) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: crystal not persisted (DEGRADED mode)".into(),
        ))
    }
    async fn get_crystal(&self, _id: &str) -> Result<Option<Crystal>, StorageError> {
        Err(StorageError::ConnectionFailed(
            "Storage unavailable (DEGRADED mode)".into(),
        ))
    }
    async fn list_crystals(&self, _limit: u32) -> Result<Vec<Crystal>, StorageError> {
        Err(StorageError::ConnectionFailed(
            "Storage unavailable (DEGRADED mode)".into(),
        ))
    }
    async fn delete_crystal(&self, _id: &str) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: cannot delete (DEGRADED mode)".into(),
        ))
    }
    async fn observe_crystal(
        &self,
        _id: &str,
        _content: &str,
        _domain: &str,
        _score: f64,
        _timestamp: &str,
        _voter_count: usize,
        _verdict_id: &str,
        _verdict_kind: &str,
    ) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: observation not persisted (DEGRADED mode)".into(),
        ))
    }
    async fn store_observation(&self, _obs: &Observation) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: observation not persisted (DEGRADED mode)".into(),
        ))
    }
    async fn query_observations(
        &self,
        _project: &str,
        _domain: Option<&str>,
        _limit: u32,
    ) -> Result<Vec<ObservationFrequency>, StorageError> {
        Err(StorageError::ConnectionFailed(
            "Storage unavailable (DEGRADED mode)".into(),
        ))
    }
    async fn query_session_targets(
        &self,
        _project: &str,
        _limit: u32,
    ) -> Result<Vec<SessionTarget>, StorageError> {
        Err(StorageError::ConnectionFailed(
            "Storage unavailable (DEGRADED mode)".into(),
        ))
    }
    async fn flush_usage(
        &self,
        _snapshot: &[(String, crate::domain::usage::DogUsage)],
    ) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: usage not flushed (DEGRADED mode)".into(),
        ))
    }
}
