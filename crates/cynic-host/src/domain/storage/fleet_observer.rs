//! Fleet storage topology observer — rollup to FleetStorageTopologyReport (cynic.fleet_storage.v1)
//! Coherence: desired (storage_topology.json) → observe (per-node Local/Remote) → reconcile → proof

use std::sync::Arc;

use cynic_storage::topology::STORAGE_TOPOLOGY_REPORT_SCHEMA;
use cynic_storage::topology::{StorageTopologyObserved, StorageTopologyReport};

use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use cynic_lifecycle::{ArgvRunner, LocalArgvRunner};

use super::local_adapter::LocalStorageTopologyAdapter;
use super::probe::{StorageTopologyObservationPort, storage_topology_report};
use super::remote_adapter::RemoteStorageTopologyAdapter;

pub struct FleetStorageTopologyObserver<R: ArgvRunner> {
    pub registry_path: String,
    pub topology_path: String,
    pub client_policy_path: String,
    pub runner: Arc<R>,
}

impl FleetStorageTopologyObserver<LocalArgvRunner> {
    pub fn new(
        registry_path: impl Into<String>,
        topology_path: impl Into<String>,
        client_policy_path: impl Into<String>,
    ) -> Self {
        Self {
            registry_path: registry_path.into(),
            topology_path: topology_path.into(),
            client_policy_path: client_policy_path.into(),
            runner: Arc::new(LocalArgvRunner),
        }
    }
}

impl<R: ArgvRunner> FleetStorageTopologyObserver<R> {
    pub fn with_runner(
        registry_path: impl Into<String>,
        topology_path: impl Into<String>,
        client_policy_path: impl Into<String>,
        runner: R,
    ) -> Self {
        Self {
            registry_path: registry_path.into(),
            topology_path: topology_path.into(),
            client_policy_path: client_policy_path.into(),
            runner: Arc::new(runner),
        }
    }

    pub fn observe(&self) -> StorageTopologyReport {
        let poll_start = std::time::Instant::now();
        let discovery = FleetDiscoveryAdapter::new(&self.registry_path).discover();
        let hostname = observed_hostname();
        let mut observed = StorageTopologyObserved {
            schema: "cynic.storage_topology_observed.v1".to_string(),
            observed_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            endpoints: vec![],
            leader: None,
            poll_cost_ms: None,
            residual_unknowns: discovery.residual_unknowns.clone(),
        };
        for declared in &discovery.nodes {
            let is_local = hostname == declared.node_id || hostname == declared.ip;
            let node_observed = if is_local {
                LocalStorageTopologyAdapter {
                    topology_path: self.topology_path.clone(),
                }
                .observe_topology()
            } else {
                let target = format!("cynic@{}", declared.ip);
                RemoteStorageTopologyAdapter {
                    ssh_target: target,
                    topology_path: self.topology_path.clone(),
                    runner: Arc::clone(&self.runner),
                }
                .observe_topology()
            };
            observed.endpoints.extend(node_observed.endpoints);
            observed
                .residual_unknowns
                .extend(node_observed.residual_unknowns);
        }
        observed.poll_cost_ms = Some(poll_start.elapsed().as_millis() as u64);
        let desired: cynic_storage::topology::StorageTopologyDesired =
            std::fs::read_to_string(&self.topology_path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default();
        let client_policy: cynic_storage::client_policy::StorageClientPolicy =
            std::fs::read_to_string(&self.client_policy_path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default();
        let mut report = storage_topology_report(&desired, &observed, &client_policy);
        report.proof_ref = Some(format!("fleet_storage:{}", observed.endpoints.len()));
        report
    }
}
