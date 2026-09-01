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

use crate::adapters::local_network::LocalNetworkAdapter;
use crate::adapters::remote_network::RemoteNetworkAdapter;
use crate::fleet_discovery::FleetDiscoveryAdapter;
use crate::runtime_layout::observed_hostname;
use crate::update_lifecycle::{ArgvRunner, LocalArgvRunner};
use cynic_protocol::network::{FLEET_NETWORK_SCHEMA, FleetNetworkNode, FleetNetworkReport};

use cynic_protocol::network::{
    DnsSynapseAssessment, FLEET_WATCHDOG_REPORT_SCHEMA, FleetWatchdogReport, FreeboxLanObservation,
    NetworkAlert, NetworkAlertLevel, ServicePortAssessment, WireguardMeshAssessment,
};
fn poll_budget_ms() -> u64 {
    std::env::var("CYNIC_POLL_BUDGET_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000)
}

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
        let poll_start = std::time::Instant::now();
        let discovery = FleetDiscoveryAdapter::new(&self.registry_path).discover();
        let hostname = observed_hostname();
        let mut nodes = Vec::with_capacity(discovery.nodes.len());
        let residual_unknowns = discovery.residual_unknowns.clone();
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

        FleetNetworkReport {
            schema: FLEET_NETWORK_SCHEMA.to_string(),
            corpus_unix_s: self.now_unix_s(),
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
/// Fleet-wide network watchdog observer aggregating multi-node probes, WireGuard mesh, and DNS.
pub struct FleetWatchdogObserver<R: ArgvRunner> {
    network_observer: FleetNetworkObserver<R>,
}

impl FleetWatchdogObserver<LocalArgvRunner> {
    pub fn new(registry_path: impl Into<String>) -> Self {
        Self::with_runner(registry_path, LocalArgvRunner)
    }
}

impl<R: ArgvRunner + Clone> FleetWatchdogObserver<R> {
    pub fn with_runner(registry_path: impl Into<String>, runner: R) -> Self {
        Self {
            network_observer: FleetNetworkObserver::with_runner(registry_path, runner),
        }
    }

    pub fn observe(
        &self,
        wireguard: WireguardMeshAssessment,
        dns: DnsSynapseAssessment,
        services: Vec<ServicePortAssessment>,
        freebox: Option<FreeboxLanObservation>,
    ) -> FleetWatchdogReport {
        let fleet_network = self.network_observer.observe();
        let total_nodes = fleet_network.total_nodes;
        let healthy_nodes = fleet_network.observed_nodes;
        let failed_nodes = fleet_network.failed_nodes;
        let degraded_nodes = total_nodes.saturating_sub(healthy_nodes + failed_nodes);

        let mut alerts = Vec::new();
        let mut max_level = NetworkAlertLevel::Ok;

        // 1. Check fleet node outcomes
        for node in &fleet_network.nodes {
            if node.outcome == "failed" {
                alerts.push(NetworkAlert {
                    level: NetworkAlertLevel::Critical,
                    component: format!("node:{}", node.node_id),
                    message: format!(
                        "Network observation unreachable for node '{}' ({})",
                        node.node_id, node.ip
                    ),
                    recovery_hint: "Verify node power state and management/synapse route"
                        .to_string(),
                });
                max_level = NetworkAlertLevel::Critical;
            }
        }

        // 2. Check WireGuard mesh
        if wireguard.dead_peers > 0 {
            alerts.push(NetworkAlert {
                level: NetworkAlertLevel::Critical,
                component: "wireguard_mesh".to_string(),
                message: format!(
                    "{} WireGuard peer(s) are dead across fleet",
                    wireguard.dead_peers
                ),
                recovery_hint: "Check remote wireguard services and overlay route".to_string(),
            });
            max_level = NetworkAlertLevel::Critical;
        }

        // 3. Check DNS
        if dns.failed_records > 0 {
            alerts.push(NetworkAlert {
                level: NetworkAlertLevel::Critical,
                component: "dns_synapse".to_string(),
                message: format!("Synapse DNS failed for {} record(s)", dns.failed_records),
                recovery_hint: "Inspect CoreDNS/dnsmasq on cynic-host-proxmox (10.10.10.1)"
                    .to_string(),
            });
            max_level = NetworkAlertLevel::Critical;
        }

        // 4. Check services
        for svc in &services {
            if !svc.reachable {
                alerts.push(NetworkAlert {
                    level: NetworkAlertLevel::Critical,
                    component: format!("service:{}", svc.service),
                    message: format!(
                        "Service '{}' ({}) is unreachable",
                        svc.service, svc.endpoint
                    ),
                    recovery_hint: format!("Inspect service unit on node '{}'", svc.node),
                });
                max_level = NetworkAlertLevel::Critical;
            }
        }

        let status = match max_level {
            NetworkAlertLevel::Ok => "ok",
            NetworkAlertLevel::Warning => "warning",
            NetworkAlertLevel::Critical => "critical",
        }
        .to_string();

        let now_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        FleetWatchdogReport {
            schema: FLEET_WATCHDOG_REPORT_SCHEMA.to_string(),
            status,
            total_nodes,
            healthy_nodes,
            degraded_nodes,
            failed_nodes,
            fleet_network,
            wireguard_mesh: wireguard,
            dns_synapse: dns,
            service_ports: services,
            freebox_lan: freebox,
            alerts,
            observed_at_unix_ms: now_unix_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};

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
