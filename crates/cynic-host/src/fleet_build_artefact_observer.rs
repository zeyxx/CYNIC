//! Fleet-wide build artefact observer.
//!
//! Mirrors `fleet_disk_observer` but for `BuildArtefactReport`. Iterates
//! declared nodes from `infra/registry.json`:
//!   - local node  → `LocalBuildArtefactAdapter` (no SSH)
//!   - remote node → `RemoteBuildArtefactAdapter` (SSH via `ArgvRunner`)

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use cynic_build::build_artefact::{
    BUILD_ARTEFACT_REPORT_SCHEMA, BuildArtefactReport, FLEET_BUILD_ARTEFACT_SCHEMA,
    FleetBuildArtefactNode, FleetBuildArtefactReport,
};

use crate::adapters::local_build_artefact::LocalBuildArtefactAdapter;
use crate::adapters::remote_build_artefact::RemoteBuildArtefactAdapter;
use crate::domain::build_artefact::port::BuildArtefactObservationPort;
use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use crate::update_lifecycle::{ArgvRunner, LocalArgvRunner};

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

    fn local_report() -> BuildArtefactReport {
        match LocalBuildArtefactAdapter.observe_build_artefacts() {
            Ok(r) => r,
            Err(e) => BuildArtefactReport {
                schema: BUILD_ARTEFACT_REPORT_SCHEMA.to_string(),
                timestamp: DateTime::<Utc>::from(SystemTime::now())
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
                physical_disks: vec![],
                filesystems: vec![],
                checkouts: vec![],
                total_checkout_bytes: 0,
                total_checkout_human: "0B".to_string(),
                target_bytes: 0,
                target_human: "0B".to_string(),
                pressure: cynic_build::build_artefact::DiskPressure {
                    avail_bytes: 0,
                    avail_human: "0B".to_string(),
                    use_pct: 0,
                    is_pressured: false,
                    root_owned_target_count: 0,
                },
                residual_unknowns: vec![format!("local_observe_failed:{e}")],
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
                let report = Self::local_report();
                observed_nodes += 1;
                FleetBuildArtefactNode::Observed {
                    node_id: declared.node_id.clone(),
                    role: declared.role.clone(),
                    ip: declared.ip.clone(),
                    report,
                }
            } else {
                let target = self.ssh_target_for(&declared.ip);
                match RemoteBuildArtefactAdapter::new(self.runner.clone(), target) {
                    Ok(adapter) => match adapter.observe_build_artefacts() {
                        Ok(report) => {
                            observed_nodes += 1;
                            FleetBuildArtefactNode::Observed {
                                node_id: declared.node_id.clone(),
                                role: declared.role.clone(),
                                ip: declared.ip.clone(),
                                report,
                            }
                        }
                        Err(e) => {
                            failed_nodes += 1;
                            FleetBuildArtefactNode::Failed {
                                node_id: declared.node_id.clone(),
                                role: declared.role.clone(),
                                ip: declared.ip.clone(),
                                error: e,
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
