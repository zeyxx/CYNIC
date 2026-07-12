use crate::adapters::local_system_probe::SysinfoMetricsSampler;
use crate::build_attestation::runtime_binary_attestation;
use crate::config::WatchdogConfig;
use crate::domain::bootstrap::l0_bootstrap_stages;
use crate::observe::{self, NodeObservation};
use crate::ports::system_probe_port::SystemMetricsPort;
use serde_json::json;

use super::host_runtime::{LocalWatchdogHostRuntime, WatchdogHostRuntimePort};
use super::kernel_bridge::{LocalWatchdogKernelProbe, WatchdogKernelProbePort};

pub(super) const OBSERVATION_SCHEMA: &str = "cynic.node_watchdog_observation.v1";

pub(super) trait WatchdogNetworkObservationPort {
    fn observe_network(&self, cfg: &WatchdogConfig) -> NodeObservation;
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct LocalWatchdogNetworkObservation;

impl WatchdogNetworkObservationPort for LocalWatchdogNetworkObservation {
    fn observe_network(&self, cfg: &WatchdogConfig) -> NodeObservation {
        observe::observe_payload(Some(&cfg.watchdog.registry_path))
    }
}

pub(super) struct WatchdogObservationInputs {
    pub(super) kernel: serde_json::Value,
    pub(super) services: Vec<serde_json::Value>,
    pub(super) services_sampled: bool,
    pub(super) disk_total_gb: f64,
    pub(super) disk_available_gb: f64,
    pub(super) network_observation_value: serde_json::Value,
    pub(super) binary_attestation: serde_json::Value,
    pub(super) hostname: String,
    pub(super) ips: Vec<String>,
    pub(super) cpu_usage_percent: f32,
    pub(super) memory_used_gb: f64,
    pub(super) memory_total_gb: f64,
    pub(super) load_average_1m: f64,
    pub(super) uptime_seconds: u64,
    pub(super) observed_at: String,
}

pub(super) struct LocalSampler {
    metrics: SysinfoMetricsSampler,
}

impl LocalSampler {
    pub(super) fn new() -> Self {
        Self {
            metrics: SysinfoMetricsSampler::new(),
        }
    }

    pub(super) async fn collect(&mut self, cfg: &WatchdogConfig) -> serde_json::Value {
        self.collect_with_ports(
            cfg,
            &LocalWatchdogHostRuntime,
            &LocalWatchdogKernelProbe,
            &LocalWatchdogNetworkObservation,
        )
        .await
    }

    pub(super) async fn collect_with_ports<Runtime, KernelProbe, NetworkObserve>(
        &mut self,
        cfg: &WatchdogConfig,
        runtime: &Runtime,
        kernel_probe: &KernelProbe,
        network_observe: &NetworkObserve,
    ) -> serde_json::Value
    where
        Runtime: WatchdogHostRuntimePort,
        KernelProbe: WatchdogKernelProbePort,
        NetworkObserve: WatchdogNetworkObservationPort,
    {
        let kernel = kernel_probe.probe(cfg).await;
        let services = cfg
            .node
            .services
            .iter()
            .map(|service| {
                json!({
                    "name": service,
                    "status": runtime.service_status(service),
                })
            })
            .collect::<Vec<_>>();
        let services_sampled = !cfg.node.services.is_empty();

        let system_metrics = self.metrics.sample_metrics();
        let network_observation = network_observe.observe_network(cfg);
        let network_observation_value = serde_json::to_value(&network_observation)
            .unwrap_or_else(|error| json!({ "serialization_error": error.to_string() }));
        let binary_attestation = serde_json::to_value(runtime_binary_attestation(Some(
            &cfg.watchdog.registry_path,
        )))
        .unwrap_or_else(|error| json!({ "serialization_error": error.to_string() }));
        build_watchdog_observation(
            cfg,
            WatchdogObservationInputs {
                kernel,
                services,
                services_sampled,
                disk_total_gb: system_metrics.disk_total_gb,
                disk_available_gb: system_metrics.disk_available_gb,
                network_observation_value,
                binary_attestation,
                hostname: runtime.hostname(),
                ips: runtime.local_ips(),
                cpu_usage_percent: system_metrics.cpu_usage_percent,
                memory_used_gb: system_metrics.memory_used_gb,
                memory_total_gb: system_metrics.memory_total_gb,
                load_average_1m: system_metrics.load_average_1m,
                uptime_seconds: system_metrics.uptime_seconds,
                observed_at: chrono::Utc::now().to_rfc3339(),
            },
        )
    }
}

pub(super) fn build_watchdog_observation(
    cfg: &WatchdogConfig,
    inputs: WatchdogObservationInputs,
) -> serde_json::Value {
    let kernel_reachable = inputs.kernel["reachable"].as_bool() == Some(true);
    let network_status = inputs
        .network_observation_value
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let network_severity = inputs
        .network_observation_value
        .get("severity")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let network_degraded = network_severity != "clear";
    let status = watchdog_status(kernel_reachable, network_degraded);

    json!({
        "schema": OBSERVATION_SCHEMA,
        "status": status,
        "node": {
            "name": cfg.node.name,
            "role": cfg.node.role,
            "constellation": cfg.node.constellation,
            "hostname": inputs.hostname,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "family": std::env::consts::FAMILY,
            "ips": inputs.ips,
        },
        "bootstrap": {
            "schema": "cynic.node_bootstrap_evidence.v1",
            "mode": "l0_watchdog",
            "stages": l0_bootstrap_stages(kernel_reachable, inputs.services_sampled),
            "binary": "cynic-node",
            "requires_dog_backend": false,
        },
        "binary_attestation": inputs.binary_attestation,
        "system": {
            "cpu_usage_percent": inputs.cpu_usage_percent,
            "memory_used_gb": inputs.memory_used_gb,
            "memory_total_gb": inputs.memory_total_gb,
            "disk_total_gb": inputs.disk_total_gb,
            "disk_available_gb": inputs.disk_available_gb,
            "load_average_1m": inputs.load_average_1m,
            "uptime_seconds": inputs.uptime_seconds,
        },
        "services": inputs.services,
        "kernel": inputs.kernel,
        "network": {
            "status": network_status,
            "severity": network_severity,
            "observation": inputs.network_observation_value,
        },
        "constellation": {
            "principle": "nodes sharing a runtime function publish comparable L0 evidence under one constellation label",
            "name": cfg.node.constellation,
            "member": cfg.node.name,
        },
        "observed_at": inputs.observed_at,
    })
}

pub(super) fn watchdog_status(kernel_reachable: bool, network_degraded: bool) -> &'static str {
    if kernel_reachable && !network_degraded {
        "ok"
    } else {
        "degraded"
    }
}
