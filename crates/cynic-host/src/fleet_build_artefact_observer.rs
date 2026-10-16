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

use crate::adapters::local_build_artefact::{
    DEFAULT_CHECKOUTS_BASE, LocalBuildArtefactAdapter,
};
use crate::adapters::remote_build_artefact::RemoteBuildArtefactAdapter;
use crate::domain::build_artefact::port::BuildArtefactObservationPort;
use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use cynic_lifecycle::{ArgvRunner, LocalArgvRunner};

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
    checkouts_base: String,
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
            checkouts_base: DEFAULT_CHECKOUTS_BASE.to_string(),
        }
    }

    pub fn with_ssh_target(mut self, template: impl Into<String>) -> Self {
        self.ssh_target_template = template.into();
        self
    }

    /// Bind the fleet observation to an explicit checkout base, propagated
    /// to the local adapter and every per-node remote adapter. The default
    /// keeps the production placement, so existing call sites are unchanged.
    pub fn with_checkouts_base(mut self, base: impl Into<String>) -> Self {
        self.checkouts_base = base.into();
        self
    }

    fn ssh_target_for(&self, ip: &str) -> String {
        self.ssh_target_template.replace("{ip}", ip)
    }

    fn local_report(&self) -> BuildArtefactReport {
        match LocalBuildArtefactAdapter::with_checkouts_base(self.checkouts_base.clone())
            .observe_build_artefacts()
        {
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
                let report = self.local_report();
                observed_nodes += 1;
                FleetBuildArtefactNode::Observed {
                    node_id: declared.node_id.clone(),
                    role: declared.role.clone(),
                    ip: declared.ip.clone(),
                    report,
                }
            } else {
                let target = self.ssh_target_for(&declared.ip);
                match RemoteBuildArtefactAdapter::new(self.runner.clone(), target).map(|a| {
                    a.with_checkouts_base(self.checkouts_base.clone())
                }) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::{ArgvOutput, ArgvRunner};

    /// A runner that records every argv it is handed and answers nothing
    /// useful, so the observer's own wiring is what the test reads back.
    #[derive(Clone)]
    struct RecordingRunner {
        seen: Arc<std::sync::Mutex<Vec<Vec<String>>>>,
    }

    impl Default for RecordingRunner {
        fn default() -> Self {
            Self {
                seen: Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }
    }

    impl ArgvRunner for RecordingRunner {
        fn run(&self, argv: &[&str]) -> ArgvOutput {
            self.seen
                .lock()
                .expect("recording lock")
                .push(argv.iter().map(|s| s.to_string()).collect());
            ArgvOutput {
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: None,
            }
        }
    }

    /// One declared node, so the fleet path reaches the remote adapter and the
    /// injected base is observable in the argv the runner recorded.
    ///
    /// The registry is a unique file under the process temp dir: a bare
    /// `tempdir()` is cleaned up on drop, and a plain relative name would
    /// resolve against the test runner's cwd rather than a known location.
    fn registry_with_one_remote_node() -> String {
        let dir = std::env::temp_dir().join(format!(
            "cynic-fleet-observer-registry-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).expect("registry dir");
        let path = dir.join("registry.json");
        std::fs::write(
            &path,
            serde_json::json!({
                "network_policy": {
                    "reservations": [
                        { "name": "declared-remote", "service": "198.51.100.7", "admin": "198.51.100.7" }
                    ]
                },
                "nodes": [
                    { "id": "declared-remote", "role": "remote-test", "services": [] }
                ]
            })
            .to_string(),
        )
        .expect("write registry");
        path.display().to_string()
    }

    #[test]
    fn override_checkouts_base_reaches_the_per_node_remote_argv() {
        let registry = registry_with_one_remote_node();
        let runner = Arc::new(RecordingRunner::default());
        let observer = FleetBuildArtefactObserver::with_runner(&registry, (*runner).clone())
            .with_ssh_target("cynic@{ip}")
            .with_checkouts_base("/srv/fleet-override/checkouts");
        // The registry node is not this host, so the remote branch is the one
        // exercised; its argv is what must carry the injected base.
        let _ = observer.observe();

        let seen = runner.seen.lock().expect("recording lock").clone();
        let rendered: Vec<String> = seen.iter().map(|argv| argv.join(" ")).collect();
        assert!(
            rendered
                .iter()
                .any(|argv| argv.contains("ls -1 /srv/fleet-override/checkouts")),
            "the fleet observer did not propagate its base into the remote argv: {rendered:?}"
        );
    }
}
