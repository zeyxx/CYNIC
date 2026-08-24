//! Fleet-wide network observation driver (mirrors `fleet_cpu_observer` /
//! `fleet_disk_observer` / `fleet_memory_observer`).
//!
//! Coherence loop: desired state (registry) → observe (per-node /proc + ip) →
//! reconcile (aggregate) → proof (`FleetNetworkReport`).
//!
//! For each declared node:
//!   - local node  → `LocalNetworkAdapter` (reads `/proc/net/dev`, runs `ip -j`)
//!   - remote node → `RemoteNetworkAdapter` (SSH `ip -j ...` + `/proc/net/dev`
//!     via injected `ArgvRunner`)

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::adapters::local_network::LocalNetworkAdapter;
use crate::adapters::remote_network::RemoteNetworkAdapter;
use crate::domain::network::NetworkObservationPort;
use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use crate::update_lifecycle::{ArgvRunner, LocalArgvRunner};
use cynic_protocol::network::{FLEET_NETWORK_SCHEMA, FleetNetworkNode, FleetNetworkReport};

pub struct FleetNetworkObserver<R: ArgvRunner> {
    registry_path: String,
    ssh_target_template: String,
    runner: Arc<R>,
}

impl FleetNetworkObserver<LocalArgvRunner> {
    pub fn new(registry_path: impl Into<String>) -> Self {
        Self::with_runner(registry_path, LocalArgvRunner)
    }
}

impl<R: ArgvRunner + Clone> FleetNetworkObserver<R> {
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

    fn now_unix_s(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    fn local_node(&self, declared_ip: &str) -> FleetNetworkNode {
        let adapter = LocalNetworkAdapter::new();
        // Port view is infallible; partial failures live in report.unknowns.
        FleetNetworkNode {
            node_id: observed_hostname(),
            role: "host".to_string(),
            ip: declared_ip.to_string(),
            outcome: "observed".to_string(),
            report: Some(adapter.observe_network()),
            error: None,
        }
    }

    pub fn observe(&self) -> FleetNetworkReport {
        let discovery = FleetDiscoveryAdapter::new(&self.registry_path).discover();
        let hostname = observed_hostname();
        let mut nodes = Vec::with_capacity(discovery.nodes.len());
        let mut residual_unknowns = discovery.residual_unknowns.clone();
        let mut observed_nodes = 0usize;
        let mut failed_nodes = 0usize;

        for declared in &discovery.nodes {
            let is_local = hostname == declared.node_id || hostname == declared.ip;
            let node: FleetNetworkNode = if is_local {
                let mut n = self.local_node(&declared.ip);
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
                match RemoteNetworkAdapter::new(self.runner.clone(), &target) {
                    Ok(adapter) => {
                        // Inherent view is fallible: transport-level failure
                        // (SSH unreachable) marks the node failed, while a
                        // successful connection with partial probes stays
                        // observed with residual unknowns.
                        match adapter.observe_network() {
                            Ok(report) => {
                                observed_nodes += 1;
                                FleetNetworkNode {
                                    node_id: declared.node_id.clone(),
                                    role: declared.role.clone(),
                                    ip: declared.ip.clone(),
                                    outcome: "observed".to_string(),
                                    report: Some(report),
                                    error: None,
                                }
                            }
                            Err(error) => {
                                failed_nodes += 1;
                                FleetNetworkNode {
                                    node_id: declared.node_id.clone(),
                                    role: declared.role.clone(),
                                    ip: declared.ip.clone(),
                                    outcome: "failed".to_string(),
                                    report: None,
                                    error: Some(error),
                                }
                            }
                        }
                    }
                    Err(_) => {
                        failed_nodes += 1;
                        FleetNetworkNode {
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

        FleetNetworkReport {
            schema: FLEET_NETWORK_SCHEMA,
            corpus_unix_s: self.now_unix_s(),
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

    const SAMPLE_DEV: &str = "Inter | Receive |  ...\n face |bytes ...\n  eth0: 1234 ...\n";

    fn registry_json(ip: &str) -> String {
        format!(
            r#"{{
              "network_policy": {{ "reservations": [{{"name":"node-{ip}","synapse":"{ip}"}}] }},
              "nodes": [{{"id":"node-{ip}","role":"worker","services":[]}}]
            }}"#
        )
    }

    fn sample_runner() -> MapArgvRunner {
        // Keys are the FULL argv the adapter issues; bare ["ssh"] keys would
        // prefix-match every call and always return the first response.
        // SAMPLE_DEV must carry >= 16 numeric columns after the interface name
        // (parse_usage_from_dev MIN_COLS = 9 rx + 8 tx fields).
        let target = "cynic@10.0.0.7";
        let ok = |out: &str| ArgvOutput {
            stdout: out.to_string(),
            exit_code: Some(0),
            stderr: String::new(),
            spawn_error: None,
        };
        let realistic_dev = concat!(
            "Inter-|   Receive                                                |  Transmit\n",
            " face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n",
            "  eth0: 123456789 987654321    0    0    0     0          0         0  987654321 123456789    0    0    0     0       0          0\n",
        );
        MapArgvRunner {
            responses: vec![
                (
                    ["ssh", target, "cat /proc/net/dev"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    ok(realistic_dev),
                ),
                (
                    ["ssh", target, "ip", "-j", "link"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    ok(
                        "[{\"ifname\":\"eth0\",\"address\":\"00:11:22:33:44:55\",\"operstate\":\"up\",\"mtu\":1500,\"speed\":1000,\"duplex\":\"full\"}]",
                    ),
                ),
                (
                    ["ssh", target, "ip", "-j", "addr"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    ok(
                        "[{\"ifname\":\"eth0\",\"addr_info\":[{\"family\":\"inet\",\"local\":\"10.0.0.7\",\"prefixlen\":24}]}]",
                    ),
                ),
                (
                    ["ssh", target, "ip", "-j", "route"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    ok("[{\"dst\":\"default\",\"gateway\":\"10.0.0.1\",\"dev\":\"eth0\"}]"),
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
        let path = std::env::temp_dir().join(format!("cynic-fleet-net-{unique}.json"));
        std::fs::write(&path, registry_json("10.0.0.7")).unwrap();

        let observer =
            FleetNetworkObserver::with_runner(path.to_string_lossy().to_string(), sample_runner());
        let report = observer.observe();
        assert_eq!(report.schema, FLEET_NETWORK_SCHEMA);
        assert_eq!(report.total_nodes, 1);
        assert_eq!(report.observed_nodes, 1);
        assert_eq!(report.failed_nodes, 0);
        match &report.nodes[0] {
            FleetNetworkNode {
                report: Some(r), ..
            } => {
                assert_eq!(r.schema, "cynic.host_network_report.v1");
                assert!(r.usage.iter().any(|u| u.iface == "eth0"));
                assert!(r.addresses.iter().any(|a| a.address == "10.0.0.7"));
                assert!(r.routes.iter().any(|rt| rt.dst == "default"));
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
        let path = std::env::temp_dir().join(format!("cynic-fleet-net-fail-{unique}.json"));
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
        let observer =
            FleetNetworkObserver::with_runner(path.to_string_lossy().to_string(), runner);
        let report = observer.observe();
        assert_eq!(report.total_nodes, 1);
        assert_eq!(report.failed_nodes, 1);
        assert_eq!(report.observed_nodes, 0);
        assert!(matches!(
            &report.nodes[0],
            FleetNetworkNode { outcome, .. } if outcome == "failed"
        ));
    }

    #[test]
    fn unreadable_registry_declare_nothing() {
        let observer = FleetNetworkObserver::with_runner(
            "/nonexistent/cynic-net-registry.json".to_string(),
            MapArgvRunner { responses: vec![] },
        );
        let report = observer.observe();
        assert_eq!(report.total_nodes, 0);
        assert!(report.nodes.is_empty());
    }
}
