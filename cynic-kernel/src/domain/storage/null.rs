//! NullStorage — no-op StoragePort adapter for graceful degradation.
//! When the real DB is unavailable, the kernel substitutes this so verdicts
//! still pass through the pipeline (but are not persisted). K14: degraded
//! mode returns explicit errors rather than silently succeeding.

use super::{
    ActivityStorage, CrystalStorage, DispatchStorage, Observation, StateLogStorage, StorageError,
    StoragePort, SubmissionStorage, TaskStorage, UsageRow, VerdictStorage,
};
use crate::domain::ccm::Crystal;
use crate::domain::dog::Verdict;
use crate::domain::verdict_queue::QueuedVerdict;
use async_trait::async_trait;

/// No-op storage for graceful degradation when DB is unavailable.
/// Verdicts pass through but are not persisted.
#[derive(Debug)]
pub struct NullStorage;

#[async_trait]
impl VerdictStorage for NullStorage {
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
    async fn recent_max_disagreements(&self, _limit: usize) -> Result<Vec<f64>, StorageError> {
        Ok(vec![])
    }
    async fn list_verdicts_by_domain(
        &self,
        _domain: &str,
        _limit: u32,
    ) -> Result<Vec<Verdict>, StorageError> {
        Err(StorageError::ConnectionFailed(
            "Storage unavailable (DEGRADED mode)".into(),
        ))
    }
    async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError> {
        Ok(None)
    }
    async fn count_verdicts(&self) -> Result<u64, StorageError> {
        Ok(0)
    }
    async fn count_verdicts_by_kind(
        &self,
    ) -> Result<std::collections::HashMap<String, u64>, StorageError> {
        Ok(std::collections::HashMap::new())
    }
}

#[async_trait]
impl CrystalStorage for NullStorage {
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
    async fn list_crystals_filtered(
        &self,
        _limit: u32,
        _domain: Option<&str>,
        _state: Option<&str>,
    ) -> Result<Vec<Crystal>, StorageError> {
        Err(StorageError::ConnectionFailed(
            "Storage unavailable (DEGRADED mode)".into(),
        ))
    }
    async fn delete_crystal(&self, _id: &str) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: cannot delete (DEGRADED mode)".into(),
        ))
    }
    async fn update_crystal_relations(
        &self,
        _id: &str,
        _relations: std::collections::BTreeMap<String, String>,
    ) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: relations not persisted (DEGRADED mode)".into(),
        ))
    }
    async fn list_crystals_for_domain(
        &self,
        _domain: &str,
        _limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        Err(StorageError::ConnectionFailed(
            "Storage unavailable (DEGRADED mode)".into(),
        ))
    }
    async fn count_crystal_observations(&self) -> Result<u64, StorageError> {
        Ok(0)
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
    async fn observe_crystal_hypha(
        &self,
        _id: &str,
        _content: &str,
        _domain: &str,
        _score: f64,
        _timestamp: &str,
        _source: &str,
        _sentiment: Option<&str>,
    ) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: hypha observation not persisted (DEGRADED mode)".into(),
        ))
    }
    async fn shatter_crystal(
        &self,
        _id: &str,
        _reason: &str,
        _source: &str,
        _timestamp: &str,
    ) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: shatter not persisted (DEGRADED mode)".into(),
        ))
    }
    async fn store_crystal_embedding(
        &self,
        _id: &str,
        _embedding: &[f32],
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn search_crystals_semantic(
        &self,
        _query_embedding: &[f32],
        _limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        Ok(vec![])
    }
    async fn find_similar_crystal(
        &self,
        _embedding: &[f32],
        _domain: &str,
        _threshold: f64,
    ) -> Result<Option<(String, f64)>, StorageError> {
        Ok(None)
    }
    async fn list_crystals_missing_embedding(
        &self,
        _limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        Ok(vec![])
    }
    async fn consolidate_duplicate_crystals(&self) -> Result<u64, StorageError> {
        Ok(0)
    }
}

#[async_trait]
impl ActivityStorage for NullStorage {
    async fn store_observation(&self, _obs: &Observation) -> Result<(), StorageError> {
        Err(StorageError::ConnectionFailed(
            "NullStorage: observation not persisted (DEGRADED mode)".into(),
        ))
    }
    async fn store_event(
        &self,
        _event: &crate::domain::storage::Event,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn fleet_stats(
        &self,
        _window_secs: u64,
        _limit: u32,
    ) -> Result<Vec<(String, u64, f64, u64, String)>, StorageError> {
        Ok(vec![])
    }
    async fn list_events(
        &self,
        _node: Option<&str>,
        _tool: Option<&str>,
        _limit: u32,
    ) -> Result<Vec<crate::domain::storage::RawEvent>, StorageError> {
        Ok(vec![])
    }
    async fn list_degraded_nodes(
        &self,
        _window_secs: u64,
        _fatal_threshold: f64,
    ) -> Result<Vec<(String, String, u64, u64)>, StorageError> {
        Ok(vec![])
    }
    async fn list_observations_raw(
        &self,
        _domain: Option<&str>,
        _agent_id: Option<&str>,
        _limit: u32,
    ) -> Result<Vec<crate::domain::storage::RawObservation>, StorageError> {
        Ok(vec![])
    }
    async fn list_observations_by_target(
        &self,
        _domain: &str,
        _target: &str,
        _limit: u32,
    ) -> Result<Vec<crate::domain::storage::RawObservation>, StorageError> {
        Ok(vec![])
    }
    async fn list_observations_by_tag(
        &self,
        _domain: &str,
        _tag: &str,
        _limit: u32,
    ) -> Result<Vec<crate::domain::storage::RawObservation>, StorageError> {
        Ok(vec![])
    }
    async fn last_observation_per_source(
        &self,
    ) -> Result<Vec<(String, String, u64)>, StorageError> {
        Ok(vec![])
    }
    async fn count_observations(&self) -> Result<u64, StorageError> {
        Ok(0)
    }
    async fn store_session_summary(
        &self,
        _summary: &crate::domain::ccm::SessionSummary,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn list_session_summaries(
        &self,
        _limit: u32,
    ) -> Result<Vec<crate::domain::ccm::SessionSummary>, StorageError> {
        Ok(vec![])
    }
    async fn get_unsummarized_sessions(
        &self,
        _min_observations: u32,
        _limit: u32,
    ) -> Result<Vec<(String, String, u32)>, StorageError> {
        Ok(vec![])
    }
    async fn get_session_observations(
        &self,
        _session_id: &str,
    ) -> Result<Vec<crate::domain::storage::RawObservation>, StorageError> {
        Ok(vec![])
    }
    async fn store_session_compliance(
        &self,
        _compliance: &crate::domain::compliance::SessionCompliance,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn list_session_compliance(
        &self,
        _limit: u32,
    ) -> Result<Vec<crate::domain::compliance::SessionCompliance>, StorageError> {
        Ok(vec![])
    }
    async fn zone_activity(
        &self,
        _path_prefixes: &[String],
        _exclude_agent: &str,
        _project_root: &str,
    ) -> Result<Vec<crate::api::rest::dispatch::AgentActivity>, StorageError> {
        Ok(vec![])
    }
}

#[async_trait]
impl TaskStorage for NullStorage {
    async fn store_agent_task(
        &self,
        _task: &crate::domain::storage::AgentTask,
    ) -> Result<String, StorageError> {
        Err(StorageError::QueryFailed(
            "agent_tasks not supported".to_string(),
        ))
    }
    async fn list_pending_agent_tasks(
        &self,
        _kind: &str,
        _domain: Option<&str>,
        _limit: u32,
    ) -> Result<Vec<crate::domain::storage::AgentTask>, StorageError> {
        Ok(vec![])
    }
    async fn list_completed_agent_tasks(
        &self,
        _kind: &str,
        _domain: Option<&str>,
        _limit: u32,
    ) -> Result<Vec<crate::domain::storage::AgentTask>, StorageError> {
        Ok(vec![])
    }
    async fn mark_agent_task_processing(&self, _task_id: &str) -> Result<(), StorageError> {
        Ok(())
    }
    async fn update_agent_task_result(
        &self,
        _task_id: &str,
        _result: Option<String>,
        _error: Option<String>,
    ) -> Result<(), StorageError> {
        Ok(())
    }
}

#[async_trait]
impl DispatchStorage for NullStorage {
    async fn store_agent_dispatch(
        &self,
        _dispatch: &crate::domain::storage::AgentDispatch,
    ) -> Result<String, StorageError> {
        Err(StorageError::QueryFailed(
            "agent_dispatch not supported".to_string(),
        ))
    }
    async fn get_active_dispatch_for_scope(
        &self,
        _scope: &str,
    ) -> Result<Option<crate::domain::storage::AgentDispatch>, StorageError> {
        Ok(None)
    }
    async fn get_dispatch(
        &self,
        _dispatch_id: &str,
    ) -> Result<Option<crate::domain::storage::AgentDispatch>, StorageError> {
        Ok(None)
    }
    async fn get_active_dispatches_for_agent(
        &self,
        _agent_id: &str,
    ) -> Result<Vec<crate::domain::storage::AgentDispatch>, StorageError> {
        Ok(vec![])
    }
    async fn update_dispatch_status(
        &self,
        _dispatch_id: &str,
        _new_status: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn update_dispatch_pr(
        &self,
        _dispatch_id: &str,
        _pr_number: u32,
    ) -> Result<(), StorageError> {
        Ok(())
    }
}

#[async_trait]
impl StateLogStorage for NullStorage {
    async fn store_state_block(
        &self,
        _block: &crate::domain::state_log::StateBlock,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn last_state_block(
        &self,
    ) -> Result<Option<crate::domain::state_log::StateBlock>, StorageError> {
        Ok(None)
    }
    async fn list_state_blocks(
        &self,
        _since: &str,
        _limit: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
        Ok(vec![])
    }
    async fn latest_state_blocks(
        &self,
        _n: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
        Ok(vec![])
    }
}

#[async_trait]
impl SubmissionStorage for NullStorage {
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
        Ok(())
    }
    async fn list_pending_verdicts(&self, _limit: u32) -> Result<Vec<QueuedVerdict>, StorageError> {
        Ok(vec![])
    }
    async fn list_submitted_verdicts(
        &self,
        _limit: u32,
    ) -> Result<Vec<QueuedVerdict>, StorageError> {
        Ok(vec![])
    }
    async fn get_queued_verdict(
        &self,
        _verdict_id: &str,
    ) -> Result<Option<QueuedVerdict>, StorageError> {
        Ok(None)
    }
    async fn update_verdict_submitted(
        &self,
        _verdict_id: &str,
        _tx_signature: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn update_verdict_confirmed(&self, _verdict_id: &str) -> Result<(), StorageError> {
        Ok(())
    }
    async fn update_verdict_failed(
        &self,
        _verdict_id: &str,
        _error_reason: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn queue_status_counts(&self) -> Result<(u32, u32, u32, u32), StorageError> {
        Ok((0, 0, 0, 0))
    }
}

#[async_trait]
impl StoragePort for NullStorage {
    async fn ping(&self) -> Result<(), StorageError> {
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
    async fn load_usage_history(&self) -> Result<Vec<UsageRow>, StorageError> {
        Ok(vec![])
    }
    async fn load_dog_stats(
        &self,
    ) -> Result<Vec<(String, crate::domain::dog_health::DogStats)>, StorageError> {
        Ok(vec![])
    }
}
