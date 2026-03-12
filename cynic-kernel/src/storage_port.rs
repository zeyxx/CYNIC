//! StoragePort v1 — domain contract for verdict persistence.
//! Domain core defines this trait. SurrealDB adapter implements it.
//! NOTE: CYNIC-ARCHITECTURE-TRUTHS.md defines a broader StoragePort (store_fact, query_facts,
//! register_trust, verify_trust). This v1 is scoped to verdict CRUD for the hackathon.
//! The full fact/trust API will extend this trait post-hackathon.

use async_trait::async_trait;
use crate::dog::Verdict;

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
    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError>;
    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError>;
    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError>;
}
