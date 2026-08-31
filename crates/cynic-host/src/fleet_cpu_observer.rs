//! Fleet-wide CPU observation driver (mirrors `fleet_disk_observer` /
//! `fleet_memory_observer`).
//!
//! For each declared node:
//!   - local node  → `LocalCpuAdapter` (reads `/proc/cpuinfo`, `/proc/stat`,
//!     `/proc/pressure/cpu` in-process)
//!   - remote node → `RemoteCpuAdapter` (SSH `cat /proc/...` via injected
//!     `ArgvRunner`)
//!
//! Coherence loop: desired state (registry) → observe (per-node /proc) →
//! reconcile (aggregate) → proof (`FleetCpuReport`).

use std::sync::Arc;

use crate::adapters::local_cpu::LocalCpuAdapter;
use crate::adapters::remote_cpu::RemoteCpuAdapter;
use crate::domain::cpu::port::CpuObservationPort;
use crate::domain::cpu::{FleetCpuNode, FleetCpuReport};
use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use crate::update_lifecycle::{ArgvRunner, LocalArgvRunner};

const FLEET_CPU_SCHEMA: &str = "cynic.fleet_cpu.v1";

fn poll_budget_ms() -> u64 {
    std::env::var("CYNIC_POLL_BUDGET_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000)
}

pub struct FleetCpuObserver<R: ArgvRunner> {
    registry_path: String,
    ssh_target_template: String,
    runner: Arc<R>,
}

impl FleetCpuObserver<LocalArgvRunner> {
    pub fn new(registry_path: impl Into<String>) -> Self {
        Self::with_runner(registry_path, LocalArgvRunner)
    }
}

impl<R: ArgvRunner> FleetCpuObserver<R> {
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

    fn local_node(declared_ip: &str) -> FleetCpuNode {
        let adapter = LocalCpuAdapter::new();
        match adapter.observe_cpu() {
            Ok(report) => FleetCpuNode {
                node_id: observed_hostname(),
                role: "host".to_string(),
                ip: declared_ip.to_string(),
                outcome: "observed".to_string(),
                report: Some(report),
                error: None,
            },
            Err(e) => FleetCpuNode {
                node_id: observed_hostname(),
                role: "host".to_string(),
                ip: declared_ip.to_string(),
                outcome: "failed".to_string(),
                report: None,
                error: Some(format!("{e:?}")),
            },
        }
    }

    pub fn observe(&self) -> FleetCpuReport {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let poll_start = std::time::Instant::now();
        let discovery = FleetDiscoveryAdapter::new(&self.registry_path).discover();
        let hostname = observed_hostname();
        let mut nodes: Vec<FleetCpuNode> = Vec::with_capacity(discovery.nodes.len());
        let residual_unknowns: Vec<String> = discovery.residual_unknowns.clone();
        let mut observed_nodes = 0usize;
        let mut failed_nodes = 0usize;

        for declared in &discovery.nodes {
            let is_local = hostname == declared.node_id || hostname == declared.ip;
            let node: FleetCpuNode = if is_local {
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
                match RemoteCpuAdapter::new(self.runner.clone(), target) {
                    Ok(adapter) => match adapter.observe_cpu() {
                        Ok(report) => {
                            observed_nodes += 1;
                            FleetCpuNode {
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
                            FleetCpuNode {
                                node_id: declared.node_id.clone(),
                                role: declared.role.clone(),
                                ip: declared.ip.clone(),
                                outcome: "failed".to_string(),
                                report: None,
                                error: Some(format!("{e:?}")),
                            }
                        }
                    },
                    Err(_) => {
                        failed_nodes += 1;
                        FleetCpuNode {
                            node_id: declared.node_id.clone(),
                            role: declared.role.clone(),
                            ip: declared.ip.clone(),
                            outcome: "failed".to_string(),
                            report: None,
                            error: Some("empty_ssh_target".to_string()),
                        }
                    }
                }
            };
            nodes.push(node);
        }

        let poll_cost_ms_val = poll_start.elapsed().as_millis() as u64;
        let mut residual_unknowns_with_pressure = residual_unknowns.clone();
        if poll_cost_ms_val > poll_budget_ms() {
            residual_unknowns_with_pressure.push("poll_cost_exceeded_budget".to_string());
        }
        let residual_unknowns = residual_unknowns_with_pressure;
        let budget = Some(cynic_protocol::task_pipeline::EnergyBudget {
            wall_time_ms: 5000,
            inference_tokens: None,
            cpu_ms: Some(5000),
            ram_bytes_s: None,
        });
        let poll_cost_ms = Some(poll_cost_ms_val);

        FleetCpuReport {
            schema: FLEET_CPU_SCHEMA.to_string(),
            corpus_unix_s: now,
            total_nodes: discovery.nodes.len(),
            observed_nodes,
            failed_nodes,
            nodes,
            residual_unknowns,
            budget,
            poll_cost_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};

    const SAMPLE_CPUINFO: &str = "\
processor\t: 0
vendor_id\t: AuthenticAMD
cpu family\t: 25
model\t: 1
model name\t: AMD EPYC 7R13
cpu cores\t: 4
physical id\t: 0
processor\t: 1
model name\t: AMD EPYC 7R13
";

    const SAMPLE_STAT: &str = "\
cpu  9432416 1234 3215683 47561286 156789 0 223456 100 0 0
";

    const SAMPLE_PRESSURE: &str = "\
some avg10=5.23 avg60=3.14 avg300=1.00 total=9432416
";

    fn registry_json(ip: &str) -> String {
        format!(
            r#"{{
              "network_policy": {{ "reservations": [{{"name":"node-{ip}","synapse":"{ip}"}}] }},
              "nodes": [{{"id":"node-{ip}","role":"worker","services":[]}}]
            }}"#
        )
    }

    fn sample_cpu_runner() -> MapArgvRunner {
        // Keys are the FULL argv the adapter issues (ssh + target + script);
        // a bare ["ssh"] key would prefix-match every call and always return
        // the first response, breaking multi-probe adapters.
        let target = "cynic@10.0.0.7";
        let ok = |out: &str| ArgvOutput {
            stdout: out.to_string(),
            exit_code: Some(0),
            stderr: String::new(),
            spawn_error: None,
        };
        MapArgvRunner {
            responses: vec![
                (
                    ["ssh", target, "cat /proc/cpuinfo"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    ok(&format!("{SAMPLE_CPUINFO}\n")),
                ),
                (
                    ["ssh", target, "cat /proc/stat"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    ok(SAMPLE_STAT),
                ),
                (
                    ["ssh", target, "cat /proc/pressure/cpu"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    ok(SAMPLE_PRESSURE),
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
        let path = std::env::temp_dir().join(format!("cynic-fleet-cpu-{unique}.json"));
        std::fs::write(&path, registry_json("10.0.0.7")).unwrap();

        let observer =
            FleetCpuObserver::with_runner(path.to_string_lossy().to_string(), sample_cpu_runner());
        let report = observer.observe();
        assert_eq!(report.schema, FLEET_CPU_SCHEMA);
        assert_eq!(report.total_nodes, 1);
        assert_eq!(report.observed_nodes, 1);
        assert_eq!(report.failed_nodes, 0);
        match &report.nodes[0] {
            FleetCpuNode {
                report: Some(r), ..
            } => {
                assert_eq!(r.schema, "cynic.host_cpu_report.v1");
                assert_eq!(r.physical.logical_cpus, 2);
                assert_eq!(r.physical.model_name, "AMD EPYC 7R13");
                assert_eq!(r.usage.user, 9432416);
                assert!(r.pressure.is_some());
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
        let path = std::env::temp_dir().join(format!("cynic-fleet-cpu-fail-{unique}.json"));
        std::fs::write(&path, registry_json("10.0.0.8")).unwrap();

        let runner = MapArgvRunner {
            responses: vec![(
                vec!["ssh".to_string()],
                ArgvOutput {
                    exit_code: Some(255),
                    stdout: String::new(),
                    stderr: "Connection refused".to_string(),
                    spawn_error: Some("spawn error".to_string()),
                },
            )],
        };
        let observer = FleetCpuObserver::with_runner(path.to_string_lossy().to_string(), runner);
        let report = observer.observe();
        assert_eq!(report.total_nodes, 1);
        assert_eq!(report.failed_nodes, 1);
        assert_eq!(report.observed_nodes, 0);
        assert!(matches!(
            &report.nodes[0],
            FleetCpuNode { outcome, .. } if outcome == "failed"
        ));
    }

    #[test]
    fn unreadable_registry_declare_nothing() {
        let observer = FleetCpuObserver::with_runner(
            "/nonexistent/cynic-cpu-registry.json".to_string(),
            MapArgvRunner { responses: vec![] },
        );
        let report = observer.observe();
        assert_eq!(report.total_nodes, 0);
        assert!(report.nodes.is_empty());
    }
}
