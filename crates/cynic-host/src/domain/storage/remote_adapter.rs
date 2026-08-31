//! Remote storage topology adapter — SSH injectable, reads via ArgvRunner

use std::sync::Arc;

use cynic_protocol::storage_topology::StorageTopologyObserved;

use super::probe::StorageTopologyObservationPort;
use crate::update_lifecycle::ArgvRunner;

pub struct RemoteStorageTopologyAdapter<R: ArgvRunner> {
    pub ssh_target: String,
    pub topology_path: String,
    pub runner: Arc<R>,
}

impl<R: ArgvRunner> StorageTopologyObservationPort for RemoteStorageTopologyAdapter<R> {
    fn observe_topology(&self) -> StorageTopologyObserved {
        let argv = ["ssh", &self.ssh_target, "cat", &self.topology_path];
        let out = self
            .runner
            .run(&argv.iter().map(|s| *s).collect::<Vec<_>>());
        let endpoints = if out.stdout.contains("surreal") {
            vec![format!("http://{}:8000", self.ssh_target)]
        } else {
            vec![]
        };
        StorageTopologyObserved {
            schema: "cynic.storage_topology_observed.v1".to_string(),
            observed_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            endpoints,
            leader: None,
            poll_cost_ms: Some(out.stdout.len() as u64),
            residual_unknowns: if out.exit_code == Some(0) {
                vec![]
            } else {
                vec!["ssh_topology_failed".to_string()]
            },
        }
    }
}
