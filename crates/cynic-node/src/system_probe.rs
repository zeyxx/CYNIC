//! SystemProbe concern: collect hardware metrics (CPU, RAM, Disk)
//! and POST them as observations to the kernel.

use cynic_protocol::{KernelEndpointPolicy, OBSERVE_PATH, kernel_url as build_kernel_url};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use tracing::{error, info, warn};

trait SystemProbePlatformPort {
    fn observe_platform(&self) -> PlatformObservation;
    fn restart_critical_vmid(&self, vmid: &str);
}

#[derive(Debug, Default, Clone, Copy)]
struct LinuxPlatformPort;

#[derive(Debug, Clone)]
struct PlatformObservation {
    zfs_health: serde_json::Value,
    proxmox_resources: serde_json::Value,
    critical_vmids_to_restart: Vec<String>,
}

#[derive(Debug, Clone)]
struct SystemMetricsSample {
    cpu_usage_percent: f32,
    memory_used_gb: f64,
    memory_total_gb: f64,
    disk_total_gb: f64,
    disk_available_gb: f64,
    load_average_1m: f64,
    uptime_seconds: u64,
}

fn parse_zpool_health(stdout: &[u8]) -> serde_json::Value {
    let text = String::from_utf8_lossy(stdout);
    let pools: Vec<_> = text
        .lines()
        .filter_map(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            if parts.len() >= 2 {
                Some(json!({ "name": parts[0], "health": parts[1] }))
            } else {
                None
            }
        })
        .collect();
    json!(pools)
}

fn critical_proxmox_vmids_to_restart(resources: &serde_json::Value) -> Vec<String> {
    resources
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|res| {
            let name = res.get("name").and_then(|v| v.as_str())?;
            let status = res.get("status").and_then(|v| v.as_str())?;
            let id = res.get("id").and_then(|v| v.as_str())?;
            if name == "cynic-vm-v2" && status == "stopped" {
                id.split('/').next_back().map(|vmid| vmid.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn system_metrics_payload(
    sample: SystemMetricsSample,
    platform: PlatformObservation,
) -> serde_json::Value {
    json!({
        "cpu_usage_percent": sample.cpu_usage_percent,
        "memory_used_gb": sample.memory_used_gb,
        "memory_total_gb": sample.memory_total_gb,
        "disk_total_gb": sample.disk_total_gb,
        "disk_available_gb": sample.disk_available_gb,
        "load_average_1m": sample.load_average_1m,
        "uptime_seconds": sample.uptime_seconds,
        "zfs_pools": platform.zfs_health,
        "proxmox_resources": platform.proxmox_resources,
    })
}

fn system_observation_payload(node_name: &str, metrics: serde_json::Value) -> serde_json::Value {
    json!({
        "tool": "SystemProbe",
        "domain": "infra",
        "status": "ok",
        "agent_id": node_name,
        "context": format!("System metrics for node {}", node_name),
        "value": metrics,
    })
}

pub(crate) async fn run_system_probe(
    client: Client,
    kernel_url: String,
    api_key: String,
    node_name: String,
    interval_secs: u64,
) {
    run_system_probe_with_platform(
        client,
        kernel_url,
        api_key,
        node_name,
        interval_secs,
        LinuxPlatformPort,
    )
    .await;
}

async fn run_system_probe_with_platform(
    client: Client,
    kernel_url: String,
    api_key: String,
    node_name: String,
    interval_secs: u64,
    platform: impl SystemProbePlatformPort,
) {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    let mut disks = Disks::new_with_refreshed_list();

    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

    // Skip first tick (instant)
    interval.tick().await;

    info!(node = %node_name, interval = interval_secs, "System probe active");

    loop {
        interval.tick().await;

        // Refresh metrics
        sys.refresh_all();
        disks.refresh(true);

        let cpu_usage = sys.global_cpu_usage();
        let mem_used = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0; // GB
        let mem_total = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0; // GB

        let mut disk_total = 0.0;
        let mut disk_available = 0.0;
        for disk in &disks {
            disk_total += disk.total_space() as f64 / 1024.0 / 1024.0 / 1024.0;
            disk_available += disk.available_space() as f64 / 1024.0 / 1024.0 / 1024.0;
        }

        let load = System::load_average().one;
        let uptime = System::uptime();
        let platform_observation = platform.observe_platform();
        for vmid in &platform_observation.critical_vmids_to_restart {
            warn!(
                "L0 Watchdog: Critical node cynic-vm-v2 is stopped. Initiating autonomous emergence (qm start {}).",
                vmid
            );
            platform.restart_critical_vmid(vmid);
        }

        let metrics = system_metrics_payload(
            SystemMetricsSample {
                cpu_usage_percent: cpu_usage,
                memory_used_gb: mem_used,
                memory_total_gb: mem_total,
                disk_total_gb: disk_total,
                disk_available_gb: disk_available,
                load_average_1m: load,
                uptime_seconds: uptime,
            },
            platform_observation,
        );

        let payload = system_observation_payload(&node_name, metrics);

        let url = match build_kernel_url(
            &kernel_url,
            OBSERVE_PATH,
            KernelEndpointPolicy {
                allow_loopback: true,
                ..KernelEndpointPolicy::default()
            },
        ) {
            Ok(url) => url,
            Err(e) => {
                error!(error = %e, "Error building kernel observation URL");
                continue;
            }
        };

        match client
            .post(&url)
            .bearer_auth(&api_key)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                // Success - silent or debug log
            }
            Ok(resp) => {
                warn!(
                    status = resp.status().as_u16(),
                    "Failed to send system observation"
                );
            }
            Err(e) => {
                error!(error = %e, "Error sending system observation");
            }
        }
    }
}

impl SystemProbePlatformPort for LinuxPlatformPort {
    fn observe_platform(&self) -> PlatformObservation {
        #[cfg(target_os = "linux")]
        {
            use cynic_host::process_observation::capture_command;

            let mut zfs_health = serde_json::Value::Null;
            let mut proxmox_resources = serde_json::Value::Null;
            let mut critical_vmids_to_restart = Vec::new();

            if let Ok(output) = capture_command("zpool", &["list", "-H", "-o", "name,health"])
                && output.success
            {
                zfs_health = parse_zpool_health(output.stdout.as_bytes());
            }

            if let Ok(output) = capture_command(
                "sudo",
                &[
                    "-n",
                    "pvesh",
                    "get",
                    "/cluster/resources",
                    "--output-format",
                    "json",
                ],
            ) && output.success
                && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&output.stdout)
            {
                critical_vmids_to_restart = critical_proxmox_vmids_to_restart(&json_val);
                proxmox_resources = json!({ "resources": &json_val });
            }

            PlatformObservation {
                zfs_health,
                proxmox_resources,
                critical_vmids_to_restart,
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            PlatformObservation {
                zfs_health: serde_json::Value::Null,
                proxmox_resources: serde_json::Value::Null,
                critical_vmids_to_restart: Vec::new(),
            }
        }
    }

    fn restart_critical_vmid(&self, vmid: &str) {
        #[cfg(target_os = "linux")]
        {
            use cynic_host::process_observation::capture_command;

            let _ = capture_command("sudo", &["-n", "qm", "start", vmid]);
        }

        #[cfg(not(target_os = "linux"))]
        let _ = vmid;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PlatformObservation, SystemMetricsSample, critical_proxmox_vmids_to_restart,
        parse_zpool_health, system_metrics_payload, system_observation_payload,
    };
    use serde_json::json;

    #[test]
    fn zpool_output_is_parsed_into_json_array() {
        let parsed = parse_zpool_health(b"tank ONLINE\nfastpool DEGRADED\n");
        assert_eq!(
            parsed,
            json!([
                {"name": "tank", "health": "ONLINE"},
                {"name": "fastpool", "health": "DEGRADED"}
            ])
        );
    }

    #[test]
    fn only_critical_stopped_kernel_vm_is_selected_for_restart() {
        let resources = json!([
            {"name": "cynic-vm-v2", "status": "stopped", "id": "qemu/101"},
            {"name": "cynic-vm-v2", "status": "running", "id": "qemu/102"},
            {"name": "other", "status": "stopped", "id": "qemu/103"}
        ]);
        assert_eq!(critical_proxmox_vmids_to_restart(&resources), vec!["101"]);
    }

    #[test]
    fn metrics_payload_builder_is_pure() {
        let payload = system_metrics_payload(
            SystemMetricsSample {
                cpu_usage_percent: 12.5,
                memory_used_gb: 4.0,
                memory_total_gb: 16.0,
                disk_total_gb: 100.0,
                disk_available_gb: 60.0,
                load_average_1m: 0.3,
                uptime_seconds: 123,
            },
            PlatformObservation {
                zfs_health: json!([{"name": "tank", "health": "ONLINE"}]),
                proxmox_resources: json!({"resources": []}),
                critical_vmids_to_restart: vec!["101".to_string()],
            },
        );

        assert_eq!(payload["cpu_usage_percent"], 12.5);
        assert_eq!(payload["zfs_pools"][0]["name"], "tank");
        assert_eq!(payload["proxmox_resources"]["resources"], json!([]));
    }

    #[test]
    fn system_observation_payload_wraps_metrics() {
        let payload = system_observation_payload("node-a", json!({"cpu_usage_percent": 1.0}));
        assert_eq!(payload["tool"], "SystemProbe");
        assert_eq!(payload["agent_id"], "node-a");
        assert_eq!(payload["value"]["cpu_usage_percent"], 1.0);
    }
}
