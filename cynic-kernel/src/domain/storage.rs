//! StoragePort v1 — domain contract for verdict persistence.
//! Domain core defines this trait. SurrealDB adapter implements it.
//! NOTE: CYNIC-ARCHITECTURE-TRUTHS.md defines a broader StoragePort (store_fact, query_facts,
//! register_trust, verify_trust). This v1 is scoped to verdict CRUD for the hackathon.
//! The full fact/trust API will extend this trait post-hackathon.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::domain::dog::Verdict;
use crate::domain::ccm::Crystal;

/// A development workflow observation — tool usage, file edits, errors.
/// Captured automatically by hooks, stored with TTL, feeds CCM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub project: String,
    pub agent_id: String,
    pub tool: String,
    pub target: String,
    pub domain: String,
    pub status: String,
    pub context: String,
    pub session_id: String,
    pub timestamp: String,
}

#[derive(Debug)]
pub enum StorageError {
    ConnectionFailed(String),
    QueryFailed(String),
    NotFound(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed(m) => write!(f, "Storage connection failed: {}", m),
            Self::QueryFailed(m) => write!(f, "Storage query failed: {}", m),
            Self::NotFound(m) => write!(f, "Not found: {}", m),
        }
    }
}

#[async_trait]
pub trait StoragePort: Send + Sync {
    /// Fast connectivity check — used by /health to verify DB is reachable.
    async fn ping(&self) -> Result<(), StorageError>;
    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError>;
    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError>;
    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError>;
    async fn store_crystal(&self, crystal: &Crystal) -> Result<(), StorageError>;
    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, StorageError>;
    async fn list_crystals(&self, limit: u32) -> Result<Vec<Crystal>, StorageError>;
    /// Atomically observe a new score for a crystal. Creates if not exists,
    /// updates running mean + observations + state in a single write.
    /// Eliminates the get→compute→store race condition.
    async fn observe_crystal(&self, id: &str, content: &str, domain: &str, score: f64, timestamp: &str) -> Result<(), StorageError>;

    /// Store a development workflow observation (tool usage, file edit, error).
    /// Fire-and-forget — callers should not block on this.
    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError>;

    /// Query observations by project, with optional domain filter.
    /// Returns top observations ordered by frequency (co-occurrence patterns).
    async fn query_observations(&self, project: &str, domain: Option<&str>, limit: u32) -> Result<Vec<serde_json::Value>, StorageError>;

    /// Query distinct targets per session — used for co-occurrence extraction.
    /// Returns rows of {session_id, target} for sessions with 2+ distinct targets.
    async fn query_session_targets(&self, project: &str, limit: u32) -> Result<Vec<serde_json::Value>, StorageError>;
}

/// No-op storage for graceful degradation when DB is unavailable.
/// Verdicts pass through but are not persisted.
pub struct NullStorage;

#[async_trait]
impl StoragePort for NullStorage {
    async fn ping(&self) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed("Storage unavailable (DEGRADED mode)".into()))
    }
    async fn store_verdict(&self, _verdict: &Verdict) -> Result<(), StorageError> {
        Ok(())
    }
    async fn get_verdict(&self, _id: &str) -> Result<Option<Verdict>, StorageError> {
        Err(StorageError::ConnectionFailed("Storage unavailable (DEGRADED mode)".into()))
    }
    async fn list_verdicts(&self, _limit: u32) -> Result<Vec<Verdict>, StorageError> {
        Err(StorageError::ConnectionFailed("Storage unavailable (DEGRADED mode)".into()))
    }
    async fn store_crystal(&self, _crystal: &Crystal) -> Result<(), StorageError> {
        Ok(())
    }
    async fn get_crystal(&self, _id: &str) -> Result<Option<Crystal>, StorageError> {
        Ok(None)
    }
    async fn list_crystals(&self, _limit: u32) -> Result<Vec<Crystal>, StorageError> {
        Ok(vec![])
    }
    async fn observe_crystal(&self, _id: &str, _content: &str, _domain: &str, _score: f64, _timestamp: &str) -> Result<(), StorageError> {
        Ok(())
    }
    async fn store_observation(&self, _obs: &Observation) -> Result<(), StorageError> {
        Ok(())
    }
    async fn query_observations(&self, _project: &str, _domain: Option<&str>, _limit: u32) -> Result<Vec<serde_json::Value>, StorageError> {
        Ok(vec![])
    }
    async fn query_session_targets(&self, _project: &str, _limit: u32) -> Result<Vec<serde_json::Value>, StorageError> {
        Ok(vec![])
    }
}
