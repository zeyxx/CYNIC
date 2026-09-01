use super::host_runtime::WatchdogHostRuntimePort;
use super::kernel_bridge::WatchdogKernelProbePort;
use super::kernel_bridge::observation_payload;
use super::sampler::{
    LocalSampler, WatchdogNetworkObservationPort, WatchdogObservationInputs,
    build_watchdog_observation, watchdog_status,
};
use crate::config::{KernelConfig, NodeConfig, WatchdogConfig, WatchdogRuntimeConfig};
use crate::domain::node_observation::NodeRuntimeCapabilities;
use crate::observe::NodeObservation;
use cynic_host::registry::DEFAULT_REGISTRY_PATH;
use serde_json::json;

fn config() -> WatchdogConfig {
    WatchdogConfig {
        node: NodeConfig {
            name: "node-a".to_string(),
            role: "kernel_primary".to_string(),
            constellation: "forge".to_string(),
            services: vec!["cynic-kernel".to_string()],
        },
        kernel: KernelConfig {
            url: "http://kernel:3030".to_string(),
            api_key_env: "CYNIC_API_KEY".to_string(),
            api_key: "secret".to_string(),
            heartbeat_interval_secs: 30,
        },
        watchdog: WatchdogRuntimeConfig {
            interval_secs: 30,
            http_timeout_secs: 5,
            registry_path: DEFAULT_REGISTRY_PATH.to_string(),
        },
    }
}

#[test]
fn watchdog_status_requires_kernel_and_clear_network() {
    assert_eq!(watchdog_status(true, false), "ok");
    assert_eq!(watchdog_status(true, true), "degraded");
    assert_eq!(watchdog_status(false, false), "degraded");
    assert_eq!(watchdog_status(false, true), "degraded");
}

#[test]
fn watchdog_observation_builder_is_pure() {
    let value = build_watchdog_observation(
        &config(),
        WatchdogObservationInputs {
            kernel: json!({"reachable": true, "http_status": 200}),
            services: vec![json!({"name": "cynic-kernel", "status": "active"})],
            services_sampled: true,
            disk_total_gb: 100.0,
            disk_available_gb: 40.0,
            network_observation_value: json!({"status": "mesh_active", "severity": "clear"}),
            binary_attestation: json!({"declaration_status": "declared"}),
            hostname: "test-node".to_string(),
            ips: vec!["10.20.0.2".to_string()],
            cpu_usage_percent: 12.5,
            memory_used_gb: 1.5,
            memory_total_gb: 8.0,
            load_average_1m: 0.4,
            uptime_seconds: 1234,
            observed_at: "1970-01-01T00:00:00+00:00".to_string(),
        },
    );

    assert_eq!(value["status"], "ok");
    assert_eq!(value["node"]["hostname"], "test-node");
    assert_eq!(value["network"]["severity"], "clear");
}

#[test]
fn watchdog_payload_builder_wraps_observation() {
    let payload = observation_payload(&config(), json!({"status": "degraded"}));
    assert_eq!(payload["status"], "degraded");
    assert_eq!(payload["target"], "node-a");
    assert_eq!(payload["domain"], "fleet.constellation");
}

#[derive(Debug, Default, Clone, Copy)]
struct FakeRuntime;

impl WatchdogHostRuntimePort for FakeRuntime {
    fn hostname(&self) -> String {
        "fake-node".to_string()
    }

    fn local_ips(&self) -> Vec<String> {
        vec!["10.20.0.9".to_string()]
    }

    fn service_status(&self, _service: &str) -> String {
        "active".to_string()
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct FakeKernelProbe;

impl WatchdogKernelProbePort for FakeKernelProbe {
    async fn probe(&self, _cfg: &WatchdogConfig) -> serde_json::Value {
        json!({"reachable": true, "http_status": 200})
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct FakeNetworkObserve;

impl WatchdogNetworkObservationPort for FakeNetworkObserve {
    fn observe_network(&self, _cfg: &WatchdogConfig) -> NodeObservation {
        crate::observe::NodeObservation::synthetic_for_tests(
            NodeRuntimeCapabilities {
                provider_fallback_active: false,
                synapse_route_active: true,
                wireguard_active: true,
                critical_dns_ok: true,
                critical_dns_canonical: true,
                critical_tcp_open: true,
            },
            None,
        )
    }
}

#[test]
fn watchdog_runtime_port_is_substitutable() {
    let runtime = FakeRuntime;
    assert_eq!(runtime.hostname(), "fake-node");
    assert_eq!(runtime.local_ips(), vec!["10.20.0.9".to_string()]);
    assert_eq!(runtime.service_status("anything"), "active");
}

#[tokio::test]
async fn sampler_collect_with_ports_is_substitutable() {
    let mut sampler = LocalSampler::new();
    let observation = sampler
        .collect_with_ports(
            &config(),
            &FakeRuntime,
            &FakeKernelProbe,
            &FakeNetworkObserve,
        )
        .await;

    assert_eq!(observation["status"], "ok");
    assert_eq!(observation["node"]["hostname"], "fake-node");
    assert_eq!(observation["kernel"]["reachable"], true);
}
#[test]
fn resolve_watchdog_services_and_peers_from_registry_is_dynamic() {
    let dir = tempfile::tempdir().unwrap();
    let registry_path = dir.path().join("registry.json");
    let json_data = json!({
        "network_policy": {
            "wireguard": {
                "nodes": [
                    { "node_id": "cortex", "overlay_ip": "10.20.0.3" },
                    { "node_id": "agent-host", "overlay_ip": "10.20.0.6" }
                ]
            }
        },
        "nodes": [
            {
                "id": "forgejo",
                "network": { "synapse": "10.10.10.101" },
                "services": [
                    { "name": "forgejo", "port": 3000 }
                ]
            },
            {
                "id": "surrealdb",
                "network": { "synapse": "10.10.10.103" },
                "services": [
                    { "name": "surrealdb", "port": 8000 }
                ]
            }
        ]
    });
    std::fs::write(
        &registry_path,
        serde_json::to_string_pretty(&json_data).unwrap(),
    )
    .unwrap();

    let svcs = super::resolve_watchdog_services(registry_path.to_str().unwrap());
    assert_eq!(svcs.len(), 2);
    assert_eq!(svcs[0].service, "forgejo");
    assert_eq!(svcs[0].endpoint, "10.10.10.101:3000");
    assert_eq!(svcs[1].service, "surrealdb");
    assert_eq!(svcs[1].endpoint, "10.10.10.103:8000");

    let peers = super::resolve_overlay_peer_ips(registry_path.to_str().unwrap(), "cortex");
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0], "10.20.0.6");
}
