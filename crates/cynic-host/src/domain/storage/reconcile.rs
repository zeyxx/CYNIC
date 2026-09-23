//! Reconcile storage topology — pure decision from desired vs observed

use cynic_storage::client_policy::StorageClientPolicy;
use cynic_storage::topology::{StorageTopologyDesired, StorageTopologyObserved};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageTopologyPlan {
    pub actions: Vec<StorageTopologyAction>,
    pub residual_unknowns: Vec<String>,
    pub proof_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageTopologyAction {
    AddReplica { desired: u32, observed: usize },
    ElectLeader,
    Backpressure { poll_cost_ms: u64, budget_ms: u64 },
    UpdateClientPolicy { endpoints: Vec<String> },
}

pub fn reconcile_storage_topology(
    desired: &StorageTopologyDesired,
    observed: &StorageTopologyObserved,
    client_policy: &StorageClientPolicy,
) -> StorageTopologyPlan {
    let mut actions = Vec::new();
    let mut residual = observed.residual_unknowns.clone();
    if (observed.endpoints.len() as u32) < desired.target_topology.replicas {
        actions.push(StorageTopologyAction::AddReplica {
            desired: desired.target_topology.replicas,
            observed: observed.endpoints.len(),
        });
        residual.push("single_endpoint_no_replication".to_string());
    }
    if observed.leader.is_none() && desired.target_topology.replicas > 1 {
        actions.push(StorageTopologyAction::ElectLeader);
    }
    if let Some(cost) = observed.poll_cost_ms {
        if cost > desired.budget.wall_time_ms {
            actions.push(StorageTopologyAction::Backpressure {
                poll_cost_ms: cost,
                budget_ms: desired.budget.wall_time_ms,
            });
            residual.push("poll_cost_exceeded_budget".to_string());
        }
    }
    if client_policy.client_policy.endpoints.len() != observed.endpoints.len() {
        actions.push(StorageTopologyAction::UpdateClientPolicy {
            endpoints: observed.endpoints.clone(),
        });
    }
    StorageTopologyPlan {
        actions,
        residual_unknowns: residual,
        proof_ref: None,
    }
}
