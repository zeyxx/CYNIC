//! Bounded apply for storage topology — explicit argv + rollback intent

use cynic_protocol::storage_client_policy::StorageClientPolicy;
use cynic_protocol::storage_topology::{StorageTopologyDesired, StorageTopologyObserved};

use super::reconcile::StorageTopologyPlan;

#[derive(Debug, Clone)]
pub struct StorageTopologyApplyIntent {
    pub actions: Vec<String>,
    pub rollback: String,
    pub proof_ref: String,
    pub mutates: bool,
}

pub fn plan_storage_topology_apply(
    desired: &StorageTopologyDesired,
    observed: &StorageTopologyObserved,
    plan: &StorageTopologyPlan,
    _client_policy: &StorageClientPolicy,
) -> StorageTopologyApplyIntent {
    let actions = plan
        .actions
        .iter()
        .map(|a| format!("{a:?}"))
        .collect::<Vec<_>>();
    let rollback = if desired.target_topology.replicas == 1 {
        "restore single endpoint".to_string()
    } else {
        format!("rollback to {} replicas", observed.endpoints.len())
    };
    StorageTopologyApplyIntent {
        actions,
        rollback,
        proof_ref: format!("storage_topology:{}", observed.endpoints.len()),
        mutates: !plan.actions.is_empty(),
    }
}
