//! Federated Task Dispatch & Synchronization Use Case.
//!
//! Wraps domain tasks in cryptographically attributed `FederatedTaskEnvelope`s,
//! issues monotonic `FederatedNodeLease`s with fencing tokens,
//! and orchestrates task distribution across reachable Synapse WireGuard mesh nodes.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use cynic_protocol::{
    DomainTask, DomainTaskStatus, FederatedNodeLease, FederatedTaskEnvelope, TaskDomain,
};
use cynic_protocol::build_node::{BuildNodeCandidate, resolve_build_node};
use serde::{Deserialize, Serialize};

use super::federated_mesh_probe_uc::probe_federated_synapse_mesh;

pub const FEDERATED_DISPATCH_RECEIPT_SCHEMA: &str = "cynic.federation.dispatch_receipt.v1";
pub const FEDERATED_SYNC_REPORT_SCHEMA: &str = "cynic.federation.sync_report.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedDispatchReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub envelope: FederatedTaskEnvelope,
    pub lease: FederatedNodeLease,
    pub target_node_id: String,
    pub target_node_ip: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedSyncReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub local_node_id: String,
    pub active_peers_count: usize,
    pub total_tasks_synced: usize,
    pub active_leases: Vec<FederatedNodeLease>,
    pub envelopes: Vec<FederatedTaskEnvelope>,
}

#[derive(Debug)]
pub struct FederatedTaskDispatcher {
    pub local_node_id: String,
    fencing_token_counter: AtomicU64,
}

impl FederatedTaskDispatcher {
    pub fn new(local_node_id: impl Into<String>) -> Self {
        Self {
            local_node_id: local_node_id.into(),
            fencing_token_counter: AtomicU64::new(1),
        }
    }

    /// Dispatch a domain task to a specific or automatically selected federated node.
    pub fn dispatch(
        &self,
        task: DomainTask,
        target_override: Option<String>,
        budget_seconds: u64,
    ) -> Result<FederatedDispatchReceipt, String> {
        let fencing_token = self.fencing_token_counter.fetch_add(1, Ordering::SeqCst);
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // 1. Select target node based on domain role or explicit override
        let resolver = super::federated_mesh_resolver::LocalRegistryMeshResolver::from_repo_infra_default(
            super::federated_mesh_probe_uc::CANONICAL_SYNAPSE_NODES,
        );
        let target_node = match target_override {
            Some(node_id) => resolver
                .resolve_all_peers()
                .into_iter()
                .find(|n| n.node_id.eq_ignore_ascii_case(&node_id))
                .ok_or_else(|| format!("Target node '{node_id}' not found in Synapse registry"))?,
            None => select_canonical_node_for_domain(task.domain),
        };

        // 2. Formulate monotonic execution lease
        let lease_id = format!("lease_{}_{}_{}", target_node.node_id, now, fencing_token);
        let lease = FederatedNodeLease {
            schema: cynic_protocol::FEDERATED_NODE_LEASE_SCHEMA.to_string(),
            lease_id,
            node_id: target_node.node_id.clone(),
            task_id: task.id.clone(),
            fencing_token,
            acquired_at_unix_s: now,
            expires_at_unix_s: now + budget_seconds,
            renew_count: 0,
        };

        // 3. Construct envelope
        let envelope_id = format!("env_{}_{}_{}", self.local_node_id, now, fencing_token);
        let envelope = FederatedTaskEnvelope::new(
            envelope_id,
            self.local_node_id.clone(),
            Some(target_node.node_id.clone()),
            task,
            fencing_token,
            budget_seconds,
        );

        Ok(FederatedDispatchReceipt {
            schema: FEDERATED_DISPATCH_RECEIPT_SCHEMA,
            status: "dispatched",
            envelope,
            lease,
            target_node_id: target_node.node_id.clone(),
            target_node_ip: target_node.addr.clone(),
            message: format!(
                "Task attributed to node '{}' (IP: {}) under fencing token {}",
                target_node.node_id, target_node.addr, fencing_token
            ),
        })
    }

    /// Perform a full cluster synchronization across active Synapse mesh nodes.
    pub fn sync_cluster(&self, pending_tasks: &[DomainTask]) -> FederatedSyncReport {
        let probe = probe_federated_synapse_mesh(&self.local_node_id);
        let mut envelopes = Vec::new();
        let mut leases = Vec::new();

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        for task in pending_tasks
            .iter()
            .filter(|t| t.status == DomainTaskStatus::Pending)
        {
            let fencing = self.fencing_token_counter.fetch_add(1, Ordering::SeqCst);
            let target_node = select_canonical_node_for_domain(task.domain);

            let lease = FederatedNodeLease {
                schema: cynic_protocol::FEDERATED_NODE_LEASE_SCHEMA.to_string(),
                lease_id: format!("lease_{}_{}_{}", target_node.node_id, now, fencing),
                node_id: target_node.node_id.clone(),
                task_id: task.id.clone(),
                fencing_token: fencing,
                acquired_at_unix_s: now,
                expires_at_unix_s: now + task.timeout_secs,
                renew_count: 0,
            };

            let envelope = FederatedTaskEnvelope::new(
                format!("env_{}_{}", self.local_node_id, fencing),
                self.local_node_id.clone(),
                Some(target_node.node_id.clone()),
                task.clone(),
                fencing,
                task.timeout_secs,
            );

            leases.push(lease);
            envelopes.push(envelope);
        }

        FederatedSyncReport {
            schema: FEDERATED_SYNC_REPORT_SCHEMA,
            status: if probe.reachable_nodes_count > 1 {
                "synced"
            } else {
                "local_only"
            },
            local_node_id: self.local_node_id.clone(),
            active_peers_count: probe.reachable_nodes_count,
            total_tasks_synced: envelopes.len(),
            active_leases: leases,
            envelopes,
        }
    }
}

fn select_canonical_node_for_domain(
    domain: TaskDomain,
) -> cynic_protocol::peer_resolver::PeerEndpoint {
    let resolver = super::federated_mesh_resolver::LocalRegistryMeshResolver::from_repo_infra_default(
        super::federated_mesh_probe_uc::CANONICAL_SYNAPSE_NODES,
    );
    let (primary_role, fallback_role) = match domain {
        TaskDomain::CodeAudit | TaskDomain::Infra => ("build_runner", Some("hypervisor")),
        TaskDomain::Governance => ("storage_primary", None),
        TaskDomain::Inference | TaskDomain::Cortex | TaskDomain::Nightshift => {
            ("cortex_primary", None)
        }
    };
    super::federated_mesh_resolver::select_peer_for_role(&resolver, primary_role)
        .or_else(|| {
            fallback_role
                .and_then(|r| super::federated_mesh_resolver::select_peer_for_role(&resolver, r))
        })
        .unwrap_or_else(|| resolver.resolve_all_peers().remove(0))
}

/// Dynamic, drift-aware build-node selection.
///
/// Replaces the hardcoded `build_runner` lookup for build domains with the
/// pure `resolve_build_node` resolver from `cynic-protocol::build_node`.
/// Inputs are observed (not declared): mesh reachability + OS-substrate
/// health (`OsHealthAnomaly` folded into `BuildNodeCandidate.health`) +
/// declared role priority.
///
/// Returns `None` when no reachable, non-OS-critical builder exists — the
/// caller MUST then emit a `TopologyDriftSignal` (drift is data, not a crash)
/// rather than falling back to a degraded node. This is the B3 wiring:
/// "mesh live primes registry seed; select by health then role priority".
pub fn select_build_node(candidates: &[BuildNodeCandidate]) -> Option<String> {
    resolve_build_node(candidates).map(|sel| sel.node_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::TaskPriority;

    #[test]
    fn test_federated_dispatcher_attribution_and_fencing() {
        let dispatcher = FederatedTaskDispatcher::new("cortex-test");
        let task = DomainTask::new(
            "task_test_123",
            TaskDomain::Inference,
            TaskPriority::Agent,
            "Benchmark Qwen 2B",
            serde_json::json!({}),
            "agent_1",
        );

        let receipt = dispatcher.dispatch(task, None, 300).unwrap();
        assert_eq!(receipt.status, "dispatched");
        assert_eq!(receipt.envelope.fencing_token, 1);
        assert_eq!(receipt.lease.fencing_token, 1);
        assert!(receipt.lease.is_valid_at(receipt.lease.acquired_at_unix_s));
    }
}
