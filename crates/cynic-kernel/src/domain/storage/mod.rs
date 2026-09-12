//! StoragePort v1 — domain contract for verdict persistence.
//! Domain core defines this trait. SurrealDB adapter implements it.
//! NOTE: CYNIC-ARCHITECTURE-TRUTHS.md defines a broader StoragePort (store_fact, query_facts,
//! register_trust, verify_trust). This v1 is scoped to verdict CRUD for the hackathon.
//! The full fact/trust API will extend this trait post-hackathon.

mod null;
mod types;

pub use null::NullStorage;
pub use types::{
    AgentDispatch, AgentTask, AuditContext, Event, MeshNode, MeshRoute, Observation, PRMetadata,
    ProofOfHistory, RawEvent, RawObservation, StorageError, StorageMetrics, SubmissionTaskContent,
    UsageRow,
};

use crate::domain::ccm::Crystal;
use crate::domain::dog::Verdict;
use crate::domain::verdict_queue::QueuedVerdict;
use async_trait::async_trait;

// ── SUB-TRAITS (Interface Segregation) ──────────────────────

#[async_trait]
pub trait VerdictStorage: Send + Sync {
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn recent_max_disagreements(&self, limit: usize) -> Result<Vec<f64>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_verdicts_by_domain(
        &self,
        domain: &str,
        limit: u32,
    ) -> Result<Vec<Verdict>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn count_verdicts(&self) -> Result<u64, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn count_verdicts_by_kind(
        &self,
    ) -> Result<std::collections::HashMap<String, u64>, StorageError>;
}

#[async_trait]
pub trait CrystalStorage: Send + Sync {
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn store_crystal(&self, crystal: &Crystal) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_crystals(&self, limit: u32) -> Result<Vec<Crystal>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_crystals_filtered(
        &self,
        limit: u32,
        domain: Option<&str>,
        state: Option<&str>,
    ) -> Result<Vec<Crystal>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn delete_crystal(&self, id: &str) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn update_crystal_relations(
        &self,
        id: &str,
        relations: std::collections::BTreeMap<String, String>,
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_crystals_for_domain(
        &self,
        domain: &str,
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn count_crystal_observations(&self) -> Result<u64, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn observe_crystal(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: i64,
        voter_count: usize,
        verdict_id: &str,
        verdict_kind: &str,
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn observe_crystal_hypha(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: i64,
        source: &str,
        sentiment: Option<&str>,
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn shatter_crystal(
        &self,
        id: &str,
        reason: &str,
        source: &str,
        timestamp: i64,
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn store_crystal_embedding(
        &self,
        id: &str,
        embedding: &[f32],
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn search_crystals_semantic(
        &self,
        query_embedding: &[f32],
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn find_similar_crystal(
        &self,
        embedding: &[f32],
        domain: &str,
        threshold: f64,
    ) -> Result<Option<(String, f64)>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_crystals_missing_embedding(
        &self,
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn consolidate_duplicate_crystals(&self) -> Result<u64, StorageError>;
}

#[async_trait]
pub trait ActivityStorage: Send + Sync {
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn store_event(&self, event: &Event) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn fleet_stats(
        &self,
        window_secs: u64,
        limit: u32,
    ) -> Result<Vec<(String, u64, f64, u64, String)>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_events(
        &self,
        node: Option<&str>,
        tool: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawEvent>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_degraded_nodes(
        &self,
        window_secs: u64,
        fatal_threshold: f64,
    ) -> Result<Vec<(String, String, u64, u64)>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_observations_raw(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError>;
    // WHY: matches database schema, with contract-level filters for portable transport layers.
    #[allow(clippy::too_many_arguments)]
    async fn list_observations_filtered(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        project: Option<&str>,
        workstream: Option<&str>,
        thread_id: Option<&str>,
        target: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_observations_by_target(
        &self,
        domain: &str,
        target: &str,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_observations_by_tag(
        &self,
        domain: &str,
        tag: &str,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn last_observation_per_source(&self)
    -> Result<Vec<(String, String, u64)>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn count_observations(&self) -> Result<u64, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn store_session_summary(
        &self,
        summary: &crate::domain::ccm::SessionSummary,
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_session_summaries(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::SessionSummary>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn get_unsummarized_sessions(
        &self,
        min_observations: u32,
        limit: u32,
    ) -> Result<Vec<(String, String, u32)>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn get_session_observations(
        &self,
        session_id: &str,
    ) -> Result<Vec<RawObservation>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn store_session_compliance(
        &self,
        compliance: &cynic_intel::compliance::SessionCompliance,
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_session_compliance(
        &self,
        limit: u32,
    ) -> Result<Vec<cynic_intel::compliance::SessionCompliance>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn zone_activity(
        &self,
        path_prefixes: &[String],
        exclude_agent: &str,
    ) -> Result<Vec<crate::api::rest::dispatch::AgentActivity>, StorageError>;
}

#[async_trait]
pub trait TaskStorage: Send + Sync {
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn store_agent_task(&self, task: &AgentTask) -> Result<String, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_pending_agent_tasks(
        &self,
        kind: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<AgentTask>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_completed_agent_tasks(
        &self,
        kind: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<AgentTask>, StorageError>;
    async fn list_overdue_agent_tasks(&self, limit: u32) -> Result<Vec<AgentTask>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn claim_agent_task(&self, task_id: &str, agent_id: &str) -> Result<bool, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn update_agent_task_result(
        &self,
        task_id: &str,
        agent_id: &str,
        result: Option<String>,
        error: Option<String>,
    ) -> Result<bool, StorageError>;
}

#[async_trait]
pub trait MeshStorage: Send + Sync {
    /// Publish a mesh node's public key (IP is static).
    async fn publish_mesh_key(
        &self,
        mesh_ip: &str,
        public_key: &str,
    ) -> Result<MeshNode, StorageError>;
    async fn list_mesh_nodes(&self) -> Result<Vec<MeshNode>, StorageError>;
    async fn list_mesh_routes(&self) -> Result<Vec<MeshRoute>, StorageError>;
}

#[async_trait]
pub trait DispatchStorage: Send + Sync {
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn store_agent_dispatch(&self, dispatch: &AgentDispatch) -> Result<String, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn get_active_dispatch_for_scope(
        &self,
        scope: &str,
    ) -> Result<Option<AgentDispatch>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn get_dispatch(&self, dispatch_id: &str) -> Result<Option<AgentDispatch>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn get_active_dispatches_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<AgentDispatch>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn update_dispatch_status(
        &self,
        dispatch_id: &str,
        new_status: &str,
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn update_dispatch_pr(
        &self,
        dispatch_id: &str,
        pr_number: u32,
    ) -> Result<(), StorageError>;
}

#[async_trait]
pub trait StateLogStorage: Send + Sync {
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn store_state_block(
        &self,
        block: &crate::domain::state_log::StateBlock,
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn last_state_block(
        &self,
    ) -> Result<Option<crate::domain::state_log::StateBlock>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_state_blocks(
        &self,
        since: &str,
        limit: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn latest_state_blocks(
        &self,
        n: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError>;
}

#[async_trait]
pub trait SubmissionStorage: Send + Sync {
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn enqueue_verdict(
        &self,
        verdict_id: &str,
        content_hash: &str,
        q_score: f64,
        score_fidelity: f64,
        score_phi: f64,
        score_verify: f64,
        score_culture: f64,
        score_burn: f64,
        score_sovereignty: f64,
        dog_count: u32,
        verdict_type: &str,
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_pending_verdicts(&self, limit: u32) -> Result<Vec<QueuedVerdict>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn list_submitted_verdicts(&self, limit: u32)
    -> Result<Vec<QueuedVerdict>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn get_queued_verdict(
        &self,
        verdict_id: &str,
    ) -> Result<Option<QueuedVerdict>, StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn update_verdict_submitted(
        &self,
        verdict_id: &str,
        tx_signature: &str,
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn update_verdict_confirmed(&self, verdict_id: &str) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn update_verdict_failed(
        &self,
        verdict_id: &str,
        error_reason: &str,
    ) -> Result<(), StorageError>;
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn queue_status_counts(&self) -> Result<(u32, u32, u32, u32), StorageError>;
}

#[async_trait]
pub trait ProofOfHistoryStorage: Send + Sync {
    async fn store_proof_of_history(
        &self,
        poh: &crate::domain::storage::ProofOfHistory,
    ) -> Result<String, StorageError>;
    async fn list_proof_of_history(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::storage::ProofOfHistory>, StorageError>;
}

// ── COMPOSITE STORAGE PORT ──────────────────────────────────

#[async_trait]
pub trait StoragePort:
    VerdictStorage
    + CrystalStorage
    + ActivityStorage
    + TaskStorage
    + MeshStorage
    + DispatchStorage
    + StateLogStorage
    + SubmissionStorage
    + ProofOfHistoryStorage
{
    /// Fast connectivity check — used by /health to verify DB is reachable.
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn ping(&self) -> Result<(), StorageError>;

    /// Storage observability — returns None if metrics not available (e.g. NullStorage).
    fn metrics(&self) -> Option<StorageMetrics> {
        None
    }

    /// Flush usage snapshot to persistent storage.
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn flush_usage(
        &self,
        snapshot: &[(String, crate::domain::usage::DogUsage)],
    ) -> Result<(), StorageError>;

    /// Load historical usage data — used to restore DogUsageTracker at boot.
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn load_usage_history(&self) -> Result<Vec<UsageRow>, StorageError> {
        Ok(vec![])
    }

    /// Flush DogStats to persistent storage. One row per Dog.
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn flush_dog_stats(
        &self,
        _stats: &[(String, crate::domain::dog_health::DogStats)],
    ) -> Result<(), StorageError> {
        Ok(())
    }

    /// Load persisted DogStats at boot — restores quality knowledge across restarts.
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn load_dog_stats(
        &self,
    ) -> Result<Vec<(String, crate::domain::dog_health::DogStats)>, StorageError> {
        Ok(vec![])
    }

    /// Load persisted DogStats for a specific Dog.
    async fn load_dog_stat(
        &self,
        _dog_id: &str,
    ) -> Result<Option<crate::domain::dog_health::DogStats>, StorageError> {
        Ok(None)
    }

    /// TTL cleanup — remove stale observations and audit entries.
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn cleanup_ttl(&self) -> Result<(), StorageError> {
        Ok(())
    }

    /// Get per-table operation metrics for data-centric measurement.
    // WHY: matches database schema
    #[allow(clippy::too_many_arguments)]
    async fn get_table_op_metrics(
        &self,
    ) -> Result<Vec<crate::storage::TableOpMetrics>, StorageError> {
        Ok(vec![])
    }

    async fn submit_agent_workload_delivery(
        &self,
        _delivery: &cynic_protocol::AgentWorkloadDelivery,
    ) -> Result<cynic_protocol::AgentWorkloadDelivery, StorageError> {
        Err(StorageError::ConnectionFailed(
            "durable agent workload delivery storage is unavailable".to_string(),
        ))
    }

    async fn agent_workload_delivery(
        &self,
        _workload_id: &str,
    ) -> Result<Option<cynic_protocol::AgentWorkloadDelivery>, StorageError> {
        Err(StorageError::ConnectionFailed(
            "durable agent workload delivery storage is unavailable".to_string(),
        ))
    }

    async fn pending_agent_workload_deliveries(
        &self,
        _node_id: &str,
    ) -> Result<Vec<cynic_protocol::AgentWorkloadDelivery>, StorageError> {
        Err(StorageError::ConnectionFailed(
            "durable agent workload delivery storage is unavailable".to_string(),
        ))
    }

    async fn accept_agent_workload_delivery(
        &self,
        _workload_id: &str,
        _node_id: &str,
        _acceptance: &cynic_protocol::AgentWorkloadDeliveryAcceptance,
    ) -> Result<cynic_protocol::AgentWorkloadDelivery, StorageError> {
        Err(StorageError::ConnectionFailed(
            "durable agent workload delivery storage is unavailable".to_string(),
        ))
    }

    async fn complete_agent_workload_delivery(
        &self,
        _publication: &cynic_protocol::AgentWorkloadCompletionPublication,
    ) -> Result<cynic_protocol::AgentWorkloadDelivery, StorageError> {
        Err(StorageError::ConnectionFailed(
            "durable agent workload completion storage is unavailable".to_string(),
        ))
    }

    async fn reserve_agent_workspace_claim(
        &self,
        _intent: &cynic_protocol::AgentWorkspaceClaimIntent,
        _admission: &cynic_protocol::AgentWorkspaceClaimAdmission,
        _now_ms: u64,
    ) -> Result<
        cynic_protocol::AgentWorkspaceClaimOutcome,
        cynic_node::ports::agent_workspace_port::AgentWorkspacePortError,
    > {
        Err(
            cynic_node::ports::agent_workspace_port::AgentWorkspacePortError::new(
                "workspace_claim_storage_unavailable",
                "durable workspace claim storage is unavailable",
            ),
        )
    }

    async fn list_agent_workspace_claims(
        &self,
        _repository_id: &str,
        _now_ms: u64,
    ) -> Result<
        Vec<cynic_protocol::AgentWorkspaceClaimObservation>,
        cynic_node::ports::agent_workspace_port::AgentWorkspacePortError,
    > {
        Err(
            cynic_node::ports::agent_workspace_port::AgentWorkspacePortError::new(
                "workspace_claim_storage_unavailable",
                "durable workspace claim storage is unavailable",
            ),
        )
    }

    async fn renew_agent_workspace_claim(
        &self,
        _guard: &cynic_protocol::LeaseGuard,
        _requested_ttl_ms: u64,
        _now_ms: u64,
    ) -> Result<
        cynic_protocol::AgentWorkspaceClaimObservation,
        cynic_node::ports::agent_workspace_port::AgentWorkspacePortError,
    > {
        Err(
            cynic_node::ports::agent_workspace_port::AgentWorkspacePortError::new(
                "workspace_claim_storage_unavailable",
                "durable workspace claim storage is unavailable",
            ),
        )
    }

    async fn release_agent_workspace_claim(
        &self,
        _guard: &cynic_protocol::LeaseGuard,
        _now_ms: u64,
    ) -> Result<(), cynic_node::ports::agent_workspace_port::AgentWorkspacePortError> {
        Err(
            cynic_node::ports::agent_workspace_port::AgentWorkspacePortError::new(
                "workspace_claim_storage_unavailable",
                "durable workspace claim storage is unavailable",
            ),
        )
    }

    async fn finalize_agent_workspace_claim(
        &self,
        _guard: &cynic_protocol::LeaseGuard,
        _proof_ref: &str,
        _now_ms: u64,
    ) -> Result<(), cynic_node::ports::agent_workspace_port::AgentWorkspacePortError> {
        Err(
            cynic_node::ports::agent_workspace_port::AgentWorkspacePortError::new(
                "workspace_claim_storage_unavailable",
                "durable workspace claim storage is unavailable",
            ),
        )
    }
}
