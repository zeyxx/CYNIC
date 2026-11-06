//! Storage topology observation ports.

use cynic_persistence::client_policy::StorageClientPolicy;
use cynic_proxmox::storage::topology::StorageTopologyObserved;

pub trait StorageTopologyObservationPort {
    fn observe_topology(&self) -> StorageTopologyObserved;
}

pub trait StorageClientPolicyObservationPort {
    fn observe_client_policy(&self) -> StorageClientPolicy;
}