//! Local storage topology adapter — reads infra/storage_topology.json + storage_client_policy.json via file I/O
//! Pure observation, no mutation — Host substrate observation layering Physical/Volume/Usage/Pressure

use std::path::Path;

use cynic_protocol::storage_client_policy::StorageClientPolicy;
use cynic_protocol::storage_topology::StorageTopologyObserved;

use super::probe::{StorageClientPolicyObservationPort, StorageTopologyObservationPort};
use std::time::Instant;

pub struct LocalStorageTopologyAdapter {
    pub topology_path: String,
}

pub struct LocalStorageClientPolicyAdapter {
    pub client_policy_path: String,
}

impl StorageClientPolicyObservationPort for LocalStorageClientPolicyAdapter {
    fn observe_client_policy(&self) -> StorageClientPolicy {
        std::fs::read_to_string(&self.client_policy_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }
}

impl StorageTopologyObservationPort for LocalStorageTopologyAdapter {
    fn observe_topology(&self) -> StorageTopologyObserved {
        let poll_start = std::time::Instant::now();
        let content = std::fs::read_to_string(&self.topology_path).unwrap_or_default();
        let endpoints = serde_json::from_str::<serde_json::Value>(&content)
            .ok()
            .and_then(|v| {
                v.get("engine")
                    .and_then(|e| e.get("backend"))
                    .and_then(|b| b.as_str())
                    .map(|_| vec!["http://127.0.0.1:8000".to_string()])
            })
            .unwrap_or_default();
        let poll_cost_ms = Some(poll_start.elapsed().as_millis() as u64);
        StorageTopologyObserved {
            schema: "cynic.storage_topology_observed.v1".to_string(),
            observed_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            endpoints,
            leader: None,
            poll_cost_ms,
            residual_unknowns: if Path::new(&self.topology_path).exists() {
                vec![]
            } else {
                vec!["topology_file_missing".to_string()]
            },
        }
    }
}
