//! StoragePort v1 — domain contract for verdict persistence.
//! Domain core defines this trait. SurrealDB adapter implements it.
//! NOTE: CYNIC-ARCHITECTURE-TRUTHS.md defines a broader StoragePort (store_fact, query_facts,
//! register_trust, verify_trust). This v1 is scoped to verdict CRUD for the hackathon.
//! The full fact/trust API will extend this trait post-hackathon.

use crate::domain::ccm::Crystal;
use crate::domain::dog::Verdict;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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

// ── TYPED QUERY RESULTS (Gate 3: zero serde_json::Value in domain/) ──

/// Aggregated observation frequency — result of GROUP BY target, tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationFrequency {
    pub target: String,
    pub tool: String,
    pub freq: u64,
}

/// Session × target pair for co-occurrence extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTarget {
    pub session_id: String,
    pub target: String,
}

/// Raw observation row — used by list_observations_raw and get_session_observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawObservation {
    /// SurrealDB record ID (e.g. "observation:abc123") — preserved for API compat.
    #[serde(default)]
    pub id: String,
    pub tool: String,
    pub target: String,
    pub domain: String,
    pub status: String,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub session_id: String,
}

/// Historical usage row loaded at boot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRow {
    pub dog_id: String,
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub requests: u64,
    #[serde(default)]
    pub failures: u64,
    #[serde(default)]
    pub total_latency_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Storage connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Storage query failed: {0}")]
    QueryFailed(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

/// Storage metrics snapshot — exposed via /health for observability.
/// Domain type — no dependency on specific DB adapter.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageMetrics {
    pub queries: u64,
    pub errors: u64,
    pub slow_queries: u64,
    pub avg_latency_ms: f64,
    pub uptime_secs: u64,
}

#[async_trait]
pub trait StoragePort: Send + Sync {
    /// Fast connectivity check — used by /health to verify DB is reachable.
    async fn ping(&self) -> Result<(), StorageError>;

    /// Storage observability — returns None if metrics not available (e.g. NullStorage).
    fn metrics(&self) -> Option<StorageMetrics> {
        None
    }
    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError>;
    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError>;
    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError>;
    async fn store_crystal(&self, crystal: &Crystal) -> Result<(), StorageError>;
    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, StorageError>;
    async fn list_crystals(&self, limit: u32) -> Result<Vec<Crystal>, StorageError>;
    /// List crystals with optional domain and state filters.
    /// Sorted by maturity (canonical > crystallized > forming) then confidence DESC.
    async fn list_crystals_filtered(
        &self,
        limit: u32,
        _domain: Option<&str>,
        _state: Option<&str>,
    ) -> Result<Vec<Crystal>, StorageError> {
        // Default: fall back to list_crystals
        self.list_crystals(limit).await
    }
    /// Delete a crystal by ID. Idempotent — no error if not found.
    async fn delete_crystal(&self, id: &str) -> Result<(), StorageError>;
    /// List mature crystals for a specific domain (including "general" cross-domain).
    /// Only returns Crystallized/Canonical state. Ordered by confidence DESC.
    /// This is the correct query for pipeline crystal injection — domain-scoped, not global top-N.
    async fn list_crystals_for_domain(
        &self,
        _domain: &str,
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        // Default: fall back to list_crystals (backward compat for NullStorage and tests)
        self.list_crystals(limit).await
    }
    /// Atomically observe a new score for a crystal. Creates if not exists,
    /// updates running mean + observations + state in a single write.
    /// Eliminates the get→compute→store race condition.
    async fn observe_crystal(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: &str,
    ) -> Result<(), StorageError>;

    /// Store a development workflow observation (tool usage, file edit, error).
    /// Fire-and-forget — callers should not block on this.
    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError>;

    /// Query observations by project, with optional domain filter.
    /// Returns top observations ordered by frequency (co-occurrence patterns).
    async fn query_observations(
        &self,
        project: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ObservationFrequency>, StorageError>;

    /// Query distinct targets per session — used for co-occurrence extraction.
    async fn query_session_targets(
        &self,
        project: &str,
        limit: u32,
    ) -> Result<Vec<SessionTarget>, StorageError>;

    /// List crystals that have no embedding vector stored.
    /// Used by the backfill task to retroactively embed orphan crystals.
    async fn list_crystals_missing_embedding(
        &self,
        _limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        Ok(vec![])
    }

    /// Count total verdicts — used to hydrate metrics on boot.
    async fn count_verdicts(&self) -> Result<u64, StorageError> {
        Ok(0)
    }

    /// Count total crystal observations — used to hydrate metrics on boot.
    async fn count_crystal_observations(&self) -> Result<u64, StorageError> {
        Ok(0)
    }

    /// List raw observations with optional filters — used by /observations endpoint.
    async fn list_observations_raw(
        &self,
        _domain: Option<&str>,
        _agent_id: Option<&str>,
        _limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        Ok(vec![])
    }

    /// Store a pre-computed embedding vector for a crystal.
    /// Enables semantic retrieval via search_crystals_semantic.
    async fn store_crystal_embedding(
        &self,
        _id: &str,
        _embedding: &[f32],
    ) -> Result<(), StorageError> {
        Ok(()) // Default no-op — adapters without vector support silently skip
    }

    /// Retrieve crystals by semantic similarity to a query embedding.
    /// Returns up to `limit` crystals ordered by cosine similarity descending.
    /// Implementations should only return Crystallized/Canonical crystals.
    async fn search_crystals_semantic(
        &self,
        _query_embedding: &[f32],
        _limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        Ok(vec![]) // Default empty — callers fall back to list_crystals
    }

    /// Find the most similar crystal to the given embedding within a domain.
    /// Searches ALL states (including Forming) — used for crystal merging during
    /// observation, not retrieval. Prevents fragmentation: "1. e4 c5" and
    /// "1. e4 c5 — Sicilian Defense" should accumulate on the same crystal.
    /// Returns (crystal_id, similarity) if a match above threshold is found.
    async fn find_similar_crystal(
        &self,
        _embedding: &[f32],
        _domain: &str,
        _threshold: f64,
    ) -> Result<Option<(String, f64)>, StorageError> {
        Ok(None) // Default: no similar crystal found — falls back to FNV hash
    }

    /// Store a session summary (compressed narrative of a development session).
    async fn store_session_summary(
        &self,
        _summary: &crate::domain::ccm::SessionSummary,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    /// List recent session summaries, ordered by created_at descending.
    async fn list_session_summaries(
        &self,
        _limit: u32,
    ) -> Result<Vec<crate::domain::ccm::SessionSummary>, StorageError> {
        Ok(vec![])
    }

    /// Get session IDs with observations but no summary yet.
    /// Returns (session_id, agent_id, observation_count) tuples.
    async fn get_unsummarized_sessions(
        &self,
        _min_observations: u32,
        _limit: u32,
    ) -> Result<Vec<(String, String, u32)>, StorageError> {
        Ok(vec![])
    }

    /// Get raw observations for a specific session (for summarization).
    async fn get_session_observations(
        &self,
        _session_id: &str,
    ) -> Result<Vec<RawObservation>, StorageError> {
        Ok(vec![])
    }

    /// Flush usage snapshot to persistent storage. The storage adapter generates
    /// the SQL/query — domain provides data only via snapshot(), no SQL in domain.
    async fn flush_usage(
        &self,
        snapshot: &[(String, crate::domain::usage::DogUsage)],
    ) -> Result<(), StorageError>;

    /// TTL cleanup — remove stale observations and audit entries.
    /// Called periodically by background tasks. Best-effort.
    async fn cleanup_ttl(&self) -> Result<(), StorageError> {
        Ok(()) // Default no-op for NullStorage
    }

    /// Get the most recent verdict's integrity hash — used to seed the hash chain at boot.
    async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError> {
        Ok(None) // Default: no chain to seed
    }

    /// Load historical usage data — used to restore DogUsageTracker at boot.
    async fn load_usage_history(&self) -> Result<Vec<UsageRow>, StorageError> {
        Ok(vec![]) // Default: no history
    }
}

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
