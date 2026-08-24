//! Fleet-wide memory observation driver.
//!
//! Mirrors `fleet_disk_observer`: reads declared nodes from
//! `infra/registry.json` (via `FleetDiscoveryAdapter`) and observes memory on
//! each node:
//!   - local node  → `LocalMemoryAdapter` (reads `/proc/meminfo` + `/proc/pressure/memory` in-process)
//!   - remote node → `RemoteMemoryAdapter` (SSH `cat /proc/...` via injected [`ArgvRunner`])
//!
//! Transport (SSH invocation) is injected: CLI supplies `LocalArgvRunner`,
//! tests inject `MapArgvRunner`. Coherence loop:
//!   desired state (registry) → observe (per-node /proc) → reconcile (aggregate) →
//!   proof (`FleetMemoryReport`).

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::adapters::local_memory::LocalMemoryAdapter;
use crate::adapters::remote_memory::RemoteMemoryAdapter;
use crate::domain::memory::port::MemoryObservationPort;
use crate::domain::memory::types::MemoryReport;
use crate::domain::memory::{FleetMemoryNode, FleetMemoryReport};
use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use crate::update_lifecycle::{ArgvRunner, LocalArgvRunner};

const FLEET_MEMORY_SCHEMA: &str = "cynic.fleet_memory.v1";

/// Fleet memory observer. Generic over the argv runner so tests inject a fake.
pub struct FleetMemoryObserver<R: ArgvRunner> {
    registry_path: String,
    ssh_target_template: String,
    runner: Arc<R>,
}

impl FleetMemoryObserver<LocalArgvRunner> {
    /// Production path: real (in-process) argv runner.
    pub fn new(registry_path: impl Into<String>) -> Self {
        Self::with_runner(registry_path, LocalArgvRunner)
    }
}

impl<R: ArgvRunner> FleetMemoryObserver<R> {
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
        let ts: DateTime<Utc> = DateTime::from(std::time::SystemTime::now());
        Some(ts.format("%Y-%m-%dT%H:%M:%SZ").to_string())
    }

    /// Observe memory on the local node (no SSH), falling back to an
    /// unobserved node on failure rather than fabricating data.
    fn local_node(declared_ip: &str) -> FleetMemoryNode {
        let local = LocalMemoryAdapter::new();
        match local.observe_memory() {
            Ok(report) => FleetMemoryNode {
                node_id: observed_hostname(),
                role: "host".to_string(),
                ip: declared_ip.to_string(),
                outcome: "observed".to_string(),
                report: Some(report),
                error: None,
            },
            Err(err) => FleetMemoryNode {
                node_id: observed_hostname(),
                role: "host".to_string(),
                ip: declared_ip.to_string(),
                outcome: "failed".to_string(),
                report: None,
                error: Some(format!("{err:?}")),
            },
        }
    }

    /// Run the full fleet observation.
    pub fn observe(&self) -> FleetMemoryReport {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let discovery = FleetDiscoveryAdapter::new(&self.registry_path).discover();
        let hostname = observed_hostname();
        let mut nodes: Vec<FleetMemoryNode> = Vec::with_capacity(discovery.nodes.len());
        let mut residual_unknowns: Vec<String> = discovery.residual_unknowns.clone();
        let mut observed_nodes = 0usize;
        let mut failed_nodes = 0usize;

        for declared in &discovery.nodes {
            let is_local = hostname == declared.node_id || hostname == declared.ip;

            let node: FleetMemoryNode = if is_local {
                let mut n = Self::local_node(&declared.ip);
                n.node_id = declared.node_id.clone();
                n.role = declared.role.clone();
                if n.report.is_some() {
                    observed_nodes += 1;
                } else {
                    failed_nodes += 1;
                }
                n
            } else {
                let target = self.ssh_target_for(&declared.ip);
                let adapter = RemoteMemoryAdapter::new(self.runner.clone(), target);
                match adapter.observe_memory() {
                    Ok(report) => {
                        observed_nodes += 1;
                        FleetMemoryNode {
                            node_id: declared.node_id.clone(),
                            role: declared.role.clone(),
                            ip: declared.ip.clone(),
                            outcome: "observed".to_string(),
                            report: Some(report),
                            error: None,
                        }
                    }
                    Err(e) => {
                        failed_nodes += 1;
                        FleetMemoryNode {
                            node_id: declared.node_id.clone(),
                            role: declared.role.clone(),
                            ip: declared.ip.clone(),
                            outcome: "failed".to_string(),
                            report: None,
                            error: Some(format!("{e:?}")),
                        }
                    }
                }
            };
            nodes.push(node);
        }

        FleetMemoryReport {
            schema: FLEET_MEMORY_SCHEMA.to_string(),
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

    const SAMPLE_MEMINFO: &str = "\
MemTotal:       16383756 kB
MemFree:         2101504 kB
MemAvailable:   13887012 kB
Buffers:          241792 kB
Cached:          8037632 kB
SwapCached:        10240 kB
SwapTotal:       8388604 kB
SwapFree:        8378364 kB
Shmem:            183364 kB
SReclaimable:      16384 kB
";

    const SAMPLE_PRESSURE: &str = "\
some avg10=0.00 avg60=0.00 avg300=0.00 total=1234567
full avg10=0.00 avg60=0.00 avg300=0.00 total=234567
";

    fn registry_json(ip: &str) -> String {
        format!(
            r#"{{
              "network_policy": {{ "reservations": [{{"name":"node-{ip}","synapse":"{ip}"}}] }},
              "nodes": [{{"id":"node-{ip}","role":"worker","services":[]}}]
            }}"#
        )
    }

    fn sample_mem_runner() -> MapArgvRunner {
        MapArgvRunner {
            responses: vec![
                (
                    vec!["ssh".to_string()],
                    ArgvOutput {
                        exit_code: Some(0),
                        stdout: format!("{SAMPLE_MEMINFO}\n{SAMPLE_PRESSURE}"),
                        stderr: String::new(),
                        spawn_error: None,
                    },
                ),
                (
                    vec!["ssh".to_string()],
                    ArgvOutput {
                        exit_code: Some(0),
                        stdout: SAMPLE_PRESSURE.to_string(),
                        stderr: String::new(),
                        spawn_error: None,
                    },
                ),
            ],
        }
    }

    #[test]
    fn observer_aggregates_remote_node_report() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir()
            .join(format!("cynic-fleet-memory-{unique}.json"));
        std::fs::write(&path, registry_json("10.0.0.7")).unwrap();

        let observer = FleetMemoryObserver::with_runner(
            path.to_string_lossy().to_string(),
            sample_mem_runner(),
        );
        let report = observer.observe();
        assert_eq!(report.schema, FLEET_MEMORY_SCHEMA);
        assert_eq!(report.total_nodes, 1);
        assert_eq!(report.observed_nodes, 1);
        assert_eq!(report.failed_nodes, 0);
        match &report.nodes[0] {
            FleetMemoryNode { report: Some(mem), .. } => {
                assert_eq!(mem.schema, "cynic.host_memory_report.v1");
                assert_eq!(mem.physical.total_bytes, 16383756 * 1024);
                assert!(mem.pressure.is_some());
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
        let path = std::env::temp_dir()
            .join(format!("cynic-fleet-memory-fail-{unique}.json"));
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
        let observer = FleetMemoryObserver::with_runner(path.to_string_lossy().to_string(), runner);
        let report = observer.observe();
        assert_eq!(report.total_nodes, 1);
        assert_eq!(report.failed_nodes, 1);
        assert_eq!(report.observed_nodes, 0);
        assert!(matches!(&report.nodes[0], FleetMemoryNode { outcome, .. } if outcome == "failed"));
    }

    #[test]
    fn unreadable_registry_declare_nothing() {
        let observer = FleetMemoryObserver::with_runner(
            "/nonexistent/cynic-memory-registry.json".to_string(),
            MapArgvRunner { responses: vec![] },
        );
        let report = observer.observe();
        assert_eq!(report.total_nodes, 0);
        assert!(report.nodes.is_empty());
    }

    // Silence unused import when the above tests are stripped.
    #[allow(dead_code)]
    fn _unused_imports(mem: MemoryReport, _t: DateTime<Utc>) -> MemoryReport {
        let _ = mem;
        mem
    }
}
