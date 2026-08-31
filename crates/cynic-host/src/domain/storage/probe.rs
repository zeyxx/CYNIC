//! Storage topology probe — pure parsers, no I/O
//! Lives in cynic_host::domain::storage::probe per Host substrate observation layering

use cynic_protocol::storage_client_policy::StorageClientPolicy;
use cynic_protocol::storage_topology::{StorageTopologyObserved, StorageTopologyReport};

pub trait StorageTopologyObservationPort {
    fn observe_topology(&self) -> StorageTopologyObserved;
}

pub trait StorageClientPolicyObservationPort {
    fn observe_client_policy(&self) -> StorageClientPolicy;
}

pub struct InMemoryStorageTopologyAdapter {
    pub observed: StorageTopologyObserved,
}

impl StorageTopologyObservationPort for InMemoryStorageTopologyAdapter {
    fn observe_topology(&self) -> StorageTopologyObserved {
        self.observed.clone()
    }
}

pub struct InMemoryStorageClientPolicyAdapter {
    pub observed: StorageClientPolicy,
}

impl StorageClientPolicyObservationPort for InMemoryStorageClientPolicyAdapter {
    fn observe_client_policy(&self) -> StorageClientPolicy {
        self.observed.clone()
    }
}

pub fn storage_topology_report(
    desired: &cynic_protocol::storage_topology::StorageTopologyDesired,
    observed: &StorageTopologyObserved,
    client_policy: &StorageClientPolicy,
) -> StorageTopologyReport {
    let desired_digest = format!("desired:{}", desired.target_topology.replicas);
    let observed_digest = format!("observed:{}", observed.endpoints.len());
    let mut residual = observed.residual_unknowns.clone();
    if observed.endpoints.len() < desired.target_topology.replicas as usize {
        residual.push("single_endpoint_no_replication".to_string());
    }
    if observed.poll_cost_ms.unwrap_or(0) > desired.budget.wall_time_ms {
        residual.push("poll_cost_exceeded_budget".to_string());
    }
    if client_policy.client_policy.endpoints.len() == 1 {
        residual.push("single_endpoint_client_policy".to_string());
    }
    StorageTopologyReport {
        schema: cynic_protocol::storage_topology::STORAGE_TOPOLOGY_REPORT_SCHEMA.to_string(),
        desired_digest,
        observed_digest,
        pre_observed_digest: None,
        post_observed_digest: None,
        actions_applied: Vec::new(),
        status: if residual.is_empty() {
            "observed".to_string()
        } else {
            "blocked".to_string()
        },
        residual_unknowns: residual,
        proof_ref: None,
    }
}
