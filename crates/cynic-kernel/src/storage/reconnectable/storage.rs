//! Reconnectable wrapper for StoragePort and core traits.

use async_trait::async_trait;
use std::sync::{Arc, RwLock};

use crate::domain::storage::{
    DispatchStorage, MeshNode, MeshRoute, MeshStorage, ProofOfHistory, ProofOfHistoryStorage,
    StateLogStorage, StorageError, StorageMetrics, StoragePort, SubmissionStorage, TaskStorage,
    UsageRow, VerdictStorage,
};
use crate::domain::verdict_queue::QueuedVerdict;

/// StoragePort proxy that can swap its inner implementation at runtime.
/// All consumers see the swap transparently — no API changes needed.
pub struct ReconnectableStorage {
    pub(super) inner: Arc<RwLock<Arc<dyn StoragePort>>>,
}

impl ReconnectableStorage {
    pub fn new(storage: Arc<dyn StoragePort>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(storage)),
        }
    }

    /// Share the lock with a StorageReconnector.
    pub fn shared_lock(&self) -> Arc<RwLock<Arc<dyn StoragePort>>> {
        Arc::clone(&self.inner)
    }

    /// Clone the current inner Arc — holds the read lock only for the clone.
    pub(super) fn current(&self) -> Arc<dyn StoragePort> {
        self.inner.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl std::fmt::Debug for ReconnectableStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReconnectableStorage").finish()
    }
}

#[async_trait]
impl VerdictStorage for ReconnectableStorage {
    async fn store_verdict(
        &self,
        verdict: &crate::domain::dog::Verdict,
    ) -> Result<(), StorageError> {
        self.current().store_verdict(verdict).await
    }
    async fn get_verdict(
        &self,
        id: &str,
    ) -> Result<Option<crate::domain::dog::Verdict>, StorageError> {
        self.current().get_verdict(id).await
    }
    async fn list_verdicts(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::dog::Verdict>, StorageError> {
        self.current().list_verdicts(limit).await
    }
    async fn recent_max_disagreements(&self, limit: usize) -> Result<Vec<f64>, StorageError> {
        self.current().recent_max_disagreements(limit).await
    }
    async fn list_verdicts_by_domain(
        &self,
        domain: &str,
        limit: u32,
    ) -> Result<Vec<crate::domain::dog::Verdict>, StorageError> {
        self.current().list_verdicts_by_domain(domain, limit).await
    }
    async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError> {
        self.current().last_integrity_hash().await
    }
    async fn count_verdicts(&self) -> Result<u64, StorageError> {
        self.current().count_verdicts().await
    }
    async fn count_verdicts_by_kind(
        &self,
    ) -> Result<std::collections::HashMap<String, u64>, StorageError> {
        self.current().count_verdicts_by_kind().await
    }
}

#[async_trait]
impl TaskStorage for ReconnectableStorage {
    async fn store_agent_task(
        &self,
        task: &crate::domain::storage::AgentTask,
    ) -> Result<String, StorageError> {
        self.current().store_agent_task(task).await
    }
    async fn list_pending_agent_tasks(
        &self,
        kind: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<crate::domain::storage::AgentTask>, StorageError> {
        self.current()
            .list_pending_agent_tasks(kind, domain, limit)
            .await
    }
    async fn list_completed_agent_tasks(
        &self,
        kind: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<crate::domain::storage::AgentTask>, StorageError> {
        self.current()
            .list_completed_agent_tasks(kind, domain, limit)
            .await
    }
    async fn list_overdue_agent_tasks(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::storage::AgentTask>, StorageError> {
        self.current().list_overdue_agent_tasks(limit).await
    }
    async fn update_agent_task_result(
        &self,
        task_id: &str,
        agent_id: &str,
        result: Option<String>,
        error: Option<String>,
    ) -> Result<bool, StorageError> {
        self.current()
            .update_agent_task_result(task_id, agent_id, result, error)
            .await
    }
    async fn claim_agent_task(&self, task_id: &str, agent_id: &str) -> Result<bool, StorageError> {
        self.current().claim_agent_task(task_id, agent_id).await
    }
}

#[async_trait]
impl MeshStorage for ReconnectableStorage {
    async fn publish_mesh_key(
        &self,
        mesh_ip: &str,
        public_key: &str,
    ) -> Result<MeshNode, StorageError> {
        self.current().publish_mesh_key(mesh_ip, public_key).await
    }
    async fn list_mesh_nodes(&self) -> Result<Vec<MeshNode>, StorageError> {
        self.current().list_mesh_nodes().await
    }
    async fn list_mesh_routes(&self) -> Result<Vec<MeshRoute>, StorageError> {
        self.current().list_mesh_routes().await
    }
}

#[async_trait]
impl DispatchStorage for ReconnectableStorage {
    async fn store_agent_dispatch(
        &self,
        dispatch: &crate::domain::storage::AgentDispatch,
    ) -> Result<String, StorageError> {
        self.current().store_agent_dispatch(dispatch).await
    }
    async fn get_active_dispatch_for_scope(
        &self,
        scope: &str,
    ) -> Result<Option<crate::domain::storage::AgentDispatch>, StorageError> {
        self.current().get_active_dispatch_for_scope(scope).await
    }
    async fn get_dispatch(
        &self,
        dispatch_id: &str,
    ) -> Result<Option<crate::domain::storage::AgentDispatch>, StorageError> {
        self.current().get_dispatch(dispatch_id).await
    }
    async fn get_active_dispatches_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<crate::domain::storage::AgentDispatch>, StorageError> {
        self.current()
            .get_active_dispatches_for_agent(agent_id)
            .await
    }
    async fn update_dispatch_status(
        &self,
        dispatch_id: &str,
        new_status: &str,
    ) -> Result<(), StorageError> {
        self.current()
            .update_dispatch_status(dispatch_id, new_status)
            .await
    }
    async fn update_dispatch_pr(
        &self,
        dispatch_id: &str,
        pr_number: u32,
    ) -> Result<(), StorageError> {
        self.current()
            .update_dispatch_pr(dispatch_id, pr_number)
            .await
    }
}

#[async_trait]
impl StateLogStorage for ReconnectableStorage {
    async fn store_state_block(
        &self,
        block: &crate::domain::state_log::StateBlock,
    ) -> Result<(), StorageError> {
        self.current().store_state_block(block).await
    }
    async fn last_state_block(
        &self,
    ) -> Result<Option<crate::domain::state_log::StateBlock>, StorageError> {
        self.current().last_state_block().await
    }
    async fn list_state_blocks(
        &self,
        since: &str,
        limit: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
        self.current().list_state_blocks(since, limit).await
    }
    async fn latest_state_blocks(
        &self,
        n: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
        self.current().latest_state_blocks(n).await
    }
}

#[async_trait]
impl SubmissionStorage for ReconnectableStorage {
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
    ) -> Result<(), StorageError> {
        self.current()
            .enqueue_verdict(
                verdict_id,
                content_hash,
                q_score,
                score_fidelity,
                score_phi,
                score_verify,
                score_culture,
                score_burn,
                score_sovereignty,
                dog_count,
                verdict_type,
            )
            .await
    }
    async fn list_pending_verdicts(&self, limit: u32) -> Result<Vec<QueuedVerdict>, StorageError> {
        self.current().list_pending_verdicts(limit).await
    }
    async fn list_submitted_verdicts(
        &self,
        limit: u32,
    ) -> Result<Vec<QueuedVerdict>, StorageError> {
        self.current().list_submitted_verdicts(limit).await
    }
    async fn get_queued_verdict(
        &self,
        verdict_id: &str,
    ) -> Result<Option<QueuedVerdict>, StorageError> {
        self.current().get_queued_verdict(verdict_id).await
    }
    async fn update_verdict_submitted(
        &self,
        verdict_id: &str,
        tx_signature: &str,
    ) -> Result<(), StorageError> {
        self.current()
            .update_verdict_submitted(verdict_id, tx_signature)
            .await
    }
    async fn update_verdict_confirmed(&self, verdict_id: &str) -> Result<(), StorageError> {
        self.current().update_verdict_confirmed(verdict_id).await
    }
    async fn update_verdict_failed(
        &self,
        verdict_id: &str,
        error_reason: &str,
    ) -> Result<(), StorageError> {
        self.current()
            .update_verdict_failed(verdict_id, error_reason)
            .await
    }
    async fn queue_status_counts(&self) -> Result<(u32, u32, u32, u32), StorageError> {
        let (p, s, c, f) = self.current().queue_status_counts().await?;
        Ok((p, s, c, f))
    }
}

#[async_trait]
impl ProofOfHistoryStorage for ReconnectableStorage {
    async fn store_proof_of_history(&self, poh: &ProofOfHistory) -> Result<String, StorageError> {
        self.current().store_proof_of_history(poh).await
    }

    async fn list_proof_of_history(&self, limit: u32) -> Result<Vec<ProofOfHistory>, StorageError> {
        self.current().list_proof_of_history(limit).await
    }
}

#[async_trait]
impl StoragePort for ReconnectableStorage {
    async fn submit_agent_workload_delivery(
        &self,
        delivery: &cynic_protocol::AgentWorkloadDelivery,
    ) -> Result<cynic_protocol::AgentWorkloadDelivery, StorageError> {
        self.current()
            .submit_agent_workload_delivery(delivery)
            .await
    }

    async fn agent_workload_delivery(
        &self,
        workload_id: &str,
    ) -> Result<Option<cynic_protocol::AgentWorkloadDelivery>, StorageError> {
        self.current().agent_workload_delivery(workload_id).await
    }

    async fn pending_agent_workload_deliveries(
        &self,
        node_id: &str,
    ) -> Result<Vec<cynic_protocol::AgentWorkloadDelivery>, StorageError> {
        self.current()
            .pending_agent_workload_deliveries(node_id)
            .await
    }

    async fn accept_agent_workload_delivery(
        &self,
        workload_id: &str,
        node_id: &str,
        acceptance: &cynic_protocol::AgentWorkloadDeliveryAcceptance,
    ) -> Result<cynic_protocol::AgentWorkloadDelivery, StorageError> {
        self.current()
            .accept_agent_workload_delivery(workload_id, node_id, acceptance)
            .await
    }

    async fn complete_agent_workload_delivery(
        &self,
        publication: &cynic_protocol::AgentWorkloadCompletionPublication,
    ) -> Result<cynic_protocol::AgentWorkloadDelivery, StorageError> {
        self.current()
            .complete_agent_workload_delivery(publication)
            .await
    }

    async fn reserve_agent_workspace_claim(
        &self,
        intent: &cynic_protocol::AgentWorkspaceClaimIntent,
        admission: &cynic_protocol::AgentWorkspaceClaimAdmission,
        now_ms: u64,
    ) -> Result<
        cynic_protocol::AgentWorkspaceClaimOutcome,
        cynic_node::ports::agent_workspace_port::AgentWorkspacePortError,
    > {
        self.current()
            .reserve_agent_workspace_claim(intent, admission, now_ms)
            .await
    }

    async fn list_agent_workspace_claims(
        &self,
        repository_id: &str,
        now_ms: u64,
    ) -> Result<
        Vec<cynic_protocol::AgentWorkspaceClaimObservation>,
        cynic_node::ports::agent_workspace_port::AgentWorkspacePortError,
    > {
        self.current()
            .list_agent_workspace_claims(repository_id, now_ms)
            .await
    }

    async fn renew_agent_workspace_claim(
        &self,
        guard: &cynic_protocol::LeaseGuard,
        requested_ttl_ms: u64,
        now_ms: u64,
    ) -> Result<
        cynic_protocol::AgentWorkspaceClaimObservation,
        cynic_node::ports::agent_workspace_port::AgentWorkspacePortError,
    > {
        self.current()
            .renew_agent_workspace_claim(guard, requested_ttl_ms, now_ms)
            .await
    }

    async fn release_agent_workspace_claim(
        &self,
        guard: &cynic_protocol::LeaseGuard,
        now_ms: u64,
    ) -> Result<(), cynic_node::ports::agent_workspace_port::AgentWorkspacePortError> {
        self.current()
            .release_agent_workspace_claim(guard, now_ms)
            .await
    }

    async fn finalize_agent_workspace_claim(
        &self,
        guard: &cynic_protocol::LeaseGuard,
        proof_ref: &str,
        now_ms: u64,
    ) -> Result<(), cynic_node::ports::agent_workspace_port::AgentWorkspacePortError> {
        self.current()
            .finalize_agent_workspace_claim(guard, proof_ref, now_ms)
            .await
    }

    async fn ping(&self) -> Result<(), StorageError> {
        self.current().ping().await
    }
    fn metrics(&self) -> Option<StorageMetrics> {
        self.current().metrics()
    }
    async fn flush_usage(
        &self,
        snapshot: &[(String, crate::domain::usage::DogUsage)],
    ) -> Result<(), StorageError> {
        self.current().flush_usage(snapshot).await
    }
    async fn load_usage_history(&self) -> Result<Vec<UsageRow>, StorageError> {
        self.current().load_usage_history().await
    }
    async fn flush_dog_stats(
        &self,
        stats: &[(String, crate::domain::dog_health::DogStats)],
    ) -> Result<(), StorageError> {
        self.current().flush_dog_stats(stats).await
    }
    async fn load_dog_stats(
        &self,
    ) -> Result<Vec<(String, crate::domain::dog_health::DogStats)>, StorageError> {
        self.current().load_dog_stats().await
    }
    async fn load_dog_stat(
        &self,
        dog_id: &str,
    ) -> Result<Option<crate::domain::dog_health::DogStats>, StorageError> {
        self.current().load_dog_stat(dog_id).await
    }
    async fn cleanup_ttl(&self) -> Result<(), StorageError> {
        self.current().cleanup_ttl().await
    }
    async fn get_table_op_metrics(
        &self,
    ) -> Result<Vec<crate::storage::TableOpMetrics>, StorageError> {
        self.current().get_table_op_metrics().await
    }
}
