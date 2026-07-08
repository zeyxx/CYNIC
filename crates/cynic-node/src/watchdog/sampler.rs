use crate::build_attestation::runtime_binary_attestation;
use crate::config::WatchdogConfig;
use crate::domain::bootstrap::l0_bootstrap_stages;
use crate::observe;
use serde_json::json;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

use super::host_runtime::{
    bytes_to_gb, disk_totals_gb, hostname, local_ips, systemd_service_status,
};
use super::kernel_bridge::probe_kernel;

pub(super) const OBSERVATION_SCHEMA: &str = "cynic.node_watchdog_observation.v1";

pub(super) struct LocalSampler {
    system: System,
    disks: Disks,
}

impl LocalSampler {
    pub(super) fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::everything())
                    .with_memory(MemoryRefreshKind::everything()),
            ),
            disks: Disks::new_with_refreshed_list(),
        }
    }

    pub(super) async fn collect(&mut self, cfg: &WatchdogConfig) -> serde_json::Value {
        self.system.refresh_all();
        self.disks.refresh(true);

        let kernel = probe_kernel(
            &cfg.kernel.url,
            &cfg.kernel.api_key,
            cfg.watchdog.http_timeout_secs,
        )
        .await;
        let services = cfg
            .node
            .services
            .iter()
            .map(|service| {
                json!({
                    "name": service,
                    "status": systemd_service_status(service),
                })
            })
            .collect::<Vec<_>>();
        let services_sampled = !cfg.node.services.is_empty();

        let disk = disk_totals_gb(&self.disks);
        let kernel_reachable = kernel["reachable"].as_bool() == Some(true);
        let network_observation = observe::observe_payload(Some(&cfg.watchdog.registry_path));
        let network_degraded = network_observation.is_warning_or_worse();
        let network_observation_value = serde_json::to_value(&network_observation)
            .unwrap_or_else(|error| json!({ "serialization_error": error.to_string() }));
        let network_status = network_observation_value
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string();
        let network_severity = network_observation_value
            .get("severity")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string();
        let status = watchdog_status(kernel_reachable, network_degraded);
        let binary_attestation = serde_json::to_value(runtime_binary_attestation(Some(
            &cfg.watchdog.registry_path,
        )))
        .unwrap_or_else(|error| json!({ "serialization_error": error.to_string() }));

        json!({
            "schema": OBSERVATION_SCHEMA,
            "status": status,
            "node": {
                "name": cfg.node.name,
                "role": cfg.node.role,
                "constellation": cfg.node.constellation,
                "hostname": hostname(),
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "family": std::env::consts::FAMILY,
                "ips": local_ips(),
            },
            "bootstrap": {
                "schema": "cynic.node_bootstrap_evidence.v1",
                "mode": "l0_watchdog",
                "stages": l0_bootstrap_stages(kernel_reachable, services_sampled),
                "binary": "cynic-node",
                "requires_dog_backend": false,
            },
            "binary_attestation": binary_attestation,
            "system": {
                "cpu_usage_percent": self.system.global_cpu_usage(),
                "memory_used_gb": bytes_to_gb(self.system.used_memory()),
                "memory_total_gb": bytes_to_gb(self.system.total_memory()),
                "disk_total_gb": disk.0,
                "disk_available_gb": disk.1,
                "load_average_1m": System::load_average().one,
                "uptime_seconds": System::uptime(),
            },
            "services": services,
            "kernel": kernel,
            "network": {
                "status": network_status,
                "severity": network_severity,
                "observation": network_observation_value,
            },
            "constellation": {
                "principle": "nodes sharing a runtime function publish comparable L0 evidence under one constellation label",
                "name": cfg.node.constellation,
                "member": cfg.node.name,
            },
            "observed_at": chrono::Utc::now().to_rfc3339(),
        })
    }
}

pub(super) fn watchdog_status(kernel_reachable: bool, network_degraded: bool) -> &'static str {
    if kernel_reachable && !network_degraded {
        "ok"
    } else {
        "degraded"
    }
}
