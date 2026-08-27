//! Fleet-wide disk observation adapter.
//!
//! Reads declared nodes from `infra/registry.json` (via the existing
//! `FleetDiscoveryAdapter`) and observes disk usage on each:
//!   - local node  → `LocalDiskAdapter` (no SSH, `df` runs in-process)
//!   - remote node → `RemoteDiskAdapter` (SSH `df` via injected [`ArgvRunner`])
//!
//! The transport (SSH invocation) is injected: the CLI supplies
//! [`LocalArgvRunner`] and tests inject a `MapArgvRunner`. This is the
//! fleet-aggregation half of the disk coherence loop that mirrors
//! `fleet_health_observer` for host substrate:
//!   desired state (registry) → observe (per-node df) → reconcile (aggregate) →
//!   proof (`FleetDiskReport`).

pub(crate) const HOST_DISK_REPORT_SCHEMA: &str = "cynic.host_disk_report.v1";

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use cynic_protocol::fleet_disk::{FLEET_DISK_SCHEMA, FleetDiskNode, FleetDiskReport};

use crate::adapters::local_disk::LocalDiskAdapter;
use crate::adapters::remote_disk::RemoteDiskAdapter;
use crate::domain::disk::DiskReport;
use crate::domain::disk::port::DiskObservationPort;
use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use crate::update_lifecycle::{ArgvRunner, LocalArgvRunner};

/// Fleet disk observer. Generic over the argv runner so tests inject a fake.
pub struct FleetDiskObserver<R: ArgvRunner> {
    registry_path: String,
    ssh_target_template: String,
    runner: Arc<R>,
}

impl FleetDiskObserver<LocalArgvRunner> {
    /// Production path: real (in-process) argv runner.
    pub fn new(registry_path: impl Into<String>) -> Self {
        Self::with_runner(registry_path, LocalArgvRunner)
    }
}

impl<R: ArgvRunner> FleetDiskObserver<R> {
    /// Build with an explicit runner (tests inject `MapArgvRunner`).
    pub fn with_runner(registry_path: impl Into<String>, runner: R) -> Self {
        Self {
            registry_path: registry_path.into(),
            ssh_target_template: "cynic@{ip}".to_string(),
            runner: Arc::new(runner),
        }
    }

    /// Set the SSH target template (default `cynic@{ip}`).
    pub fn with_ssh_target(mut self, template: impl Into<String>) -> Self {
        self.ssh_target_template = template.into();
        self
    }

    fn ssh_target_for(&self, ip: &str) -> String {
        self.ssh_target_template.replace("{ip}", ip)
    }

    fn now_isoh() -> Option<String> {
        let ts: DateTime<Utc> = DateTime::from(SystemTime::now());
        Some(ts.format("%Y-%m-%dT%H:%M:%SZ").to_string())
    }

    /// Observe disk on the local node (no SSH), falling back to an empty
    /// report on failure rather than fabricating data.
    fn local_node_report() -> DiskReport {
        match LocalDiskAdapter.observe_disk() {
            Ok(report) => report,
            Err(err) => DiskReport {
                schema: HOST_DISK_REPORT_SCHEMA.to_string(),
                timestamp: Self::now_isoh().unwrap_or_default(),
                filesystems: Vec::new(),
                known_caches: Vec::new(),
                large_files: Vec::new(),
                physical_disks: Vec::new(),
            },
        }
    }

    /// Run the full fleet observation.
    pub fn observe(&self) -> FleetDiskReport {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let discovery = FleetDiscoveryAdapter::new(&self.registry_path).discover();
        let hostname = observed_hostname();
        let mut nodes: Vec<FleetDiskNode> = Vec::with_capacity(discovery.nodes.len());
        let mut residual_unknowns: Vec<String> = Vec::new();
        let mut observed_nodes = 0usize;
        let mut failed_nodes = 0usize;

        for declared in &discovery.nodes {
            let is_local = hostname == declared.node_id || hostname == declared.ip;

            let node: FleetDiskNode = if is_local {
                let report = Self::local_node_report();
                observed_nodes += 1;
                FleetDiskNode::observed {
                    node_id: declared.node_id.clone(),
                    role: declared.role.clone(),
                    ip: declared.ip.clone(),
                    report,
                }
            } else {
                let target = self.ssh_target_for(&declared.ip);
                match RemoteDiskAdapter::new(self.runner.clone(), target) {
                    Ok(adapter) => match adapter.observe_disk() {
                        Ok(report) => {
                            observed_nodes += 1;
                            FleetDiskNode::observed {
                                node_id: declared.node_id.clone(),
                                role: declared.role.clone(),
                                ip: declared.ip.clone(),
                                report,
                            }
                        }
                        Err(e) => {
                            failed_nodes += 1;
                            FleetDiskNode::failed {
                                node_id: declared.node_id.clone(),
                                role: declared.role.clone(),
                                ip: declared.ip.clone(),
                                error: e,
                            }
                        }
                    },
                    Err(e) => {
                        failed_nodes += 1;
                        FleetDiskNode::failed {
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

        residual_unknowns.extend(discovery.residual_unknowns);

        FleetDiskReport {
            schema: FLEET_DISK_SCHEMA.to_string(),
            corpus_unix_s: now,
            total_nodes: discovery.nodes.len(),
            observed_nodes,
            failed_nodes,
            nodes,
            residual_unknowns,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};

    const SAMPLE_DF: &str = "\
Filesystem     Type  Size  Used Avail Use% Mounted on
/dev/sda2      ext4   50G   30G   20G  60% /
tmpfs          tmpfs  3.9G     0  3.9G   0% /dev
";

    fn registry_json(ip: &str) -> String {
        format!(
            r#"{{
              "network_policy": {{ "reservations": [{{"name":"node-{ip}","synapse":"{ip}"}}] }},
              "nodes": [{{"id":"node-{ip}","role":"worker","services":[]}}]
            }}"#
        )
    }

    fn stub_ssh_runner() -> MapArgvRunner {
        MapArgvRunner {
            responses: vec![(
                vec!["ssh".to_string()],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: SAMPLE_DF.to_string(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        }
    }

    #[test]
    fn observer_aggregates_remote_node_report() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cynic-fleet-disk-{unique}.json"));
        std::fs::write(&path, registry_json("10.0.0.7")).unwrap();

        // Hostname will never equal "node-10.0.0.7" locally → treated as remote.
        let observer =
            FleetDiskObserver::with_runner(path.to_string_lossy().to_string(), stub_ssh_runner());
        let report = observer.observe();
        assert_eq!(report.schema, FLEET_DISK_SCHEMA);
        assert_eq!(report.total_nodes, 1);
        assert_eq!(report.observed_nodes, 1);
        assert_eq!(report.failed_nodes, 0);
        match &report.nodes[0] {
            FleetDiskNode::observed { report, .. } => {
                assert_eq!(report.filesystems.len(), 2);
                assert_eq!(report.filesystems[0].source, "/dev/sda2");
            }
            other => panic!("expected observed node, got {other:?}"),
        }
    }

    #[test]
    fn observer_surfaces_ssh_failure() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cynic-fleet-disk-fail-{unique}.json"));
        std::fs::write(&path, registry_json("10.0.0.8")).unwrap();

        let runner = MapArgvRunner {
            responses: vec![(
                vec!["ssh".to_string()],
                ArgvOutput {
                    exit_code: Some(255),
                    stdout: String::new(),
                    stderr: "Connection refused".to_string(),
                    spawn_error: None,
                },
            )],
        };
        let observer = FleetDiskObserver::with_runner(path.to_string_lossy().to_string(), runner);
        let report = observer.observe();
        assert_eq!(report.total_nodes, 1);
        assert_eq!(report.failed_nodes, 1);
        assert_eq!(report.observed_nodes, 0);
        assert!(
            matches!(&report.nodes[0], FleetDiskNode::failed { error, .. } if error.contains("remote_df_failed"))
        );
    }

    #[test]
    fn unreadable_registry_declare_nothing() {
        let observer = FleetDiskObserver::with_runner(
            "/nonexistent/cynic-registry.json".to_string(),
            stub_ssh_runner(),
        );
        let report = observer.observe();
        assert_eq!(report.total_nodes, 0);
        assert_eq!(report.nodes.len(), 0);
    }
}
