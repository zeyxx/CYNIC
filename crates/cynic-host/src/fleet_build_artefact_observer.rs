//! Fleet-wide build artefact observer.
//!
//! Mirrors `fleet_disk_observer` but for `RetentionObservation`. Iterates
//! declared nodes from `infra/registry.json`:
//!   - local node  → `LocalBuildArtefactAdapter` (no SSH)
//!   - remote node → `RemoteBuildArtefactAdapter` (SSH via `ArgvRunner`)

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use cynic_build::build_artefact::{
    FLEET_BUILD_ARTEFACT_SCHEMA, FleetBuildArtefactNode, FleetBuildArtefactReport,
};

use crate::adapters::local_build_artefact::LocalBuildArtefactAdapter;
use crate::adapters::remote_build_artefact::RemoteBuildArtefactAdapter;
use crate::domain::build_artefact::RetentionObserver;
use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use cynic_lifecycle::{ArgvRunner, LocalArgvRunner};
use cynic_protocol::retention::RetentionError;

fn poll_budget_ms() -> u64 {
    std::env::var("CYNIC_POLL_BUDGET_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000)
}

pub struct FleetBuildArtefactObserver<R: ArgvRunner> {
    registry_path: String,
    ssh_target_template: String,
    runner: Arc<R>,
}

impl FleetBuildArtefactObserver<LocalArgvRunner> {
    pub fn new(registry_path: impl Into<String>) -> Self {
        Self::with_runner(registry_path, LocalArgvRunner)
    }
}

impl<R: ArgvRunner> FleetBuildArtefactObserver<R> {
    pub fn with_runner(registry_path: impl Into<String>, runner: R) -> Self {
        Self {
            registry_path: registry_path.into(),
            ssh_target_template: "cynic@{ip}".to_string(),
            runner: Arc::new(runner),
        }
    }

    pub fn with_ssh_target(mut self, template: impl Into<String>) -> Self {
        self.ssh_target_template = template.into();
        self
    }

    fn ssh_target_for(&self, ip: &str) -> String {
        self.ssh_target_template.replace("{ip}", ip)
    }

    fn local_observation() -> cynic_protocol::retention::RetentionObservation {
        match LocalBuildArtefactAdapter.observe() {
            Ok(o) => o,
            Err(e) => cynic_protocol::retention::RetentionObservation {
                schema: cynic_protocol::retention::RETENTION_OBSERVATION_SCHEMA.to_string(),
                timestamp: DateTime::<Utc>::from(SystemTime::now())
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
                items: vec![],
                total_reclaimable_bytes: 0,
                pressure: None,
            },
        }
    }

    pub fn observe(&self) -> FleetBuildArtefactReport {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let poll_start = std::time::Instant::now();
        let discovery = FleetDiscoveryAdapter::new(&self.registry_path).discover();
        let hostname = observed_hostname();
        let mut nodes = Vec::with_capacity(discovery.nodes.len());
        let mut observed_nodes = 0usize;
        let mut failed_nodes = 0usize;

        for declared in &discovery.nodes {
            let is_local = hostname == declared.node_id || hostname == declared.ip;
            let node = if is_local {
                let obs = Self::local_observation();
                observed_nodes += 1;
                FleetBuildArtefactNode::Observed {
                    node_id: declared.node_id.clone(),
                    role: declared.role.clone(),
                    ip: declared.ip.clone(),
                    report: obs,
                }
            } else {
                let target = self.ssh_target_for(&declared.ip);
                match RemoteBuildArtefactAdapter::new(self.runner.clone(), target) {
                    Ok(adapter) => match adapter.observe() {
                        Ok(obs) => {
                            observed_nodes += 1;
                            FleetBuildArtefactNode::Observed {
                                node_id: declared.node_id.clone(),
                                role: declared.role.clone(),
                                ip: declared.ip.clone(),
                                report: obs,
                            }
                        }
                        Err(e) => {
                            failed_nodes += 1;
                            FleetBuildArtefactNode::Failed {
                                node_id: declared.node_id.clone(),
                                role: declared.role.clone(),
                                ip: declared.ip.clone(),
                                error: e.to_string(),
                            }
                        }
                    },
                    Err(e) => {
                        failed_nodes += 1;
                        FleetBuildArtefactNode::Failed {
                            node_id: declared.node_id.clone(),
                            role: declared.role.clone(),
                            ip: declared.ip.clone(),
                            error: e,
                        }
                    }
                }
            };
            nodes.push(node);
        }

        let poll_cost_ms_val = poll_start.elapsed().as_millis() as u64;
        let mut residual_unknowns = discovery.residual_unknowns;
        if poll_cost_ms_val > poll_budget_ms() {
            residual_unknowns.push("poll_cost_exceeded_budget".to_string());
        }

        FleetBuildArtefactReport {
            schema: FLEET_BUILD_ARTEFACT_SCHEMA.to_string(),
            corpus_unix_s: now,
            total_nodes: discovery.nodes.len(),
            observed_nodes,
            failed_nodes,
            nodes,
            residual_unknowns,
            budget: Some(cynic_protocol::task_pipeline::EnergyBudget {
                wall_time_ms: 5000,
                inference_tokens: None,
                cpu_ms: Some(5000),
                ram_bytes_s: None,
            }),
            poll_cost_ms: Some(poll_cost_ms_val),
        }
    }
}
