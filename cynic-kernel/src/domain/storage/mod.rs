//! StoragePort v1 — domain contract for verdict persistence.
//! Domain core defines this trait. SurrealDB adapter implements it.
//! NOTE: CYNIC-ARCHITECTURE-TRUTHS.md defines a broader StoragePort (store_fact, query_facts,
//! register_trust, verify_trust). This v1 is scoped to verdict CRUD for the hackathon.
//! The full fact/trust API will extend this trait post-hackathon.

mod null;
mod types;

pub use null::NullStorage;
pub use types::{AgentTask, Observation, RawObservation, StorageError, StorageMetrics, UsageRow};

use crate::domain::ccm::Crystal;
use crate::domain::dog::Verdict;
use crate::domain::verdict_queue::QueuedVerdict;
use async_trait::async_trait;

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
    ///
    /// T5+T8: `voter_count` is the number of Dogs that contributed to this observation.
    /// Adapter MUST reject if `voter_count < MIN_QUORUM` — this prevents single-Dog
    /// verdicts and direct REST callers from crystallizing content.
    ///
    /// `verdict_id` links this observation to its source verdict (provenance trail).
    #[allow(clippy::too_many_arguments)]
    // WHY: observe_crystal carries 8 primitive/scalar arguments that form the crystal's
    // observation input. verdict_kind enables 4D polarity tracking.
    async fn observe_crystal(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: &str,
        voter_count: usize,
        verdict_id: &str,
        verdict_kind: &str,
    ) -> Result<(), StorageError>;

    /// Store a development workflow observation (tool usage, file edit, error).
    /// Fire-and-forget — callers should not block on this.
    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError>;

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

    /// Last observation timestamp per agent_id — used by state_log for organ liveness.
    /// Returns Vec<(agent_id, last_created_at, observation_count)>.
    async fn last_observation_per_source(
        &self,
    ) -> Result<Vec<(String, String, u64)>, StorageError> {
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

    /// Store a session compliance report.
    async fn store_session_compliance(
        &self,
        _compliance: &crate::domain::compliance::SessionCompliance,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    /// List recent compliance reports, ordered by created_at descending.
    async fn list_session_compliance(
        &self,
        _limit: u32,
    ) -> Result<Vec<crate::domain::compliance::SessionCompliance>, StorageError> {
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

    // ── Organ stats persistence (B5 — amnesia fix) ──────────

    /// Flush DogStats to persistent storage. One row per Dog.
    async fn flush_dog_stats(
        &self,
        _stats: &[(String, crate::organ::health::DogStats)],
    ) -> Result<(), StorageError> {
        Ok(()) // Default no-op for NullStorage
    }

    /// Load persisted DogStats at boot — restores quality knowledge across restarts.
    async fn load_dog_stats(
        &self,
    ) -> Result<Vec<(String, crate::organ::health::DogStats)>, StorageError> {
        Ok(vec![]) // Default: no history (K14: starts pessimistic)
    }

    /// Consolidate duplicate crystals — find crystals with identical (domain, content),
    /// merge observation counts and confidence into the survivor (most observations),
    /// delete duplicates. Returns number of duplicates removed.
    /// One-shot cleanup to fix historical fragmentation from FNV hash collisions.
    async fn consolidate_duplicate_crystals(&self) -> Result<u64, StorageError> {
        Ok(0) // Default no-op for NullStorage/MemoryStorage
    }

    // ── Agent Task Queue (K15: executor framework) ──────────

    /// Store a new agent task. Returns the task ID.
    async fn store_agent_task(&self, _task: &AgentTask) -> Result<String, StorageError> {
        Err(StorageError::QueryFailed(
            "agent_tasks not supported".to_string(),
        ))
    }

    /// List pending agent tasks for a specific kind (e.g., "hermes", "nightshift").
    async fn list_pending_agent_tasks(
        &self,
        _kind: &str,
        _limit: u32,
    ) -> Result<Vec<AgentTask>, StorageError> {
        Ok(vec![])
    }

    /// Update agent task result. Sets status to "completed" or "failed" + result/error.
    async fn update_agent_task_result(
        &self,
        _task_id: &str,
        _result: Option<String>,
        _error: Option<String>,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    /// Mark agent task as processing (status = "processing").
    async fn mark_agent_task_processing(&self, _task_id: &str) -> Result<(), StorageError> {
        Ok(())
    }

    // ── State Log (hash-chained organism state) ──────────

    /// Append a state block to the log. Blocks are append-only — never updated.
    async fn store_state_block(
        &self,
        _block: &crate::domain::state_log::StateBlock,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    /// Get the most recent state block (for chain continuation).
    async fn last_state_block(
        &self,
    ) -> Result<Option<crate::domain::state_log::StateBlock>, StorageError> {
        Ok(None)
    }

    /// List state blocks since a given timestamp, ordered by seq ASC.
    async fn list_state_blocks(
        &self,
        _since: &str,
        _limit: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
        Ok(vec![])
    }

    // ── Verdict Submission Queue (K15: Helius credit tracking + onchain submission) ──────────

    /// Enqueue a verdict with q_score >= 0.618 for onchain submission.
    /// Rejects if q_score < 0.618 (below phi⁻¹ threshold).
    /// Stores all axiom scores and metadata needed for Pinocchio submission.
    #[allow(clippy::too_many_arguments)]
    // WHY: enqueue_verdict captures all submission metadata in a single write to prevent
    // race conditions: axiom scores, verdict type, and dog count all needed for onchain instruction.
    async fn enqueue_verdict(
        &self,
        _verdict_id: &str,
        _content_hash: &str,
        _q_score: f64,
        _score_fidelity: f64,
        _score_phi: f64,
        _score_verify: f64,
        _score_culture: f64,
        _score_burn: f64,
        _score_sovereignty: f64,
        _dog_count: u32,
        _verdict_type: &str,
    ) -> Result<(), StorageError> {
        Ok(()) // Default no-op for NullStorage
    }

    /// List verdicts pending submission (status = "pending"), ordered by created_at ASC.
    async fn list_pending_verdicts(&self, _limit: u32) -> Result<Vec<QueuedVerdict>, StorageError> {
        Ok(vec![])
    }

    /// Get a queued verdict by ID.
    async fn get_queued_verdict(
        &self,
        _verdict_id: &str,
    ) -> Result<Option<QueuedVerdict>, StorageError> {
        Ok(None)
    }

    /// Mark a verdict as submitted with tx_signature. Increments retry_count and sets submitted_at.
    async fn update_verdict_submitted(
        &self,
        _verdict_id: &str,
        _tx_signature: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    /// Mark a verdict as confirmed onchain. Sets status = "confirmed" and confirmed_at timestamp.
    async fn update_verdict_confirmed(&self, _verdict_id: &str) -> Result<(), StorageError> {
        Ok(())
    }

    /// Mark a verdict as failed with error reason. Sets status = "failed" if retry_count >= 3.
    async fn update_verdict_failed(
        &self,
        _verdict_id: &str,
        _error_reason: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    /// Get counts of queued verdicts by status: (pending, submitted, confirmed, failed).
    async fn queue_status_counts(&self) -> Result<(u32, u32, u32, u32), StorageError> {
        Ok((0, 0, 0, 0))
    }
}
