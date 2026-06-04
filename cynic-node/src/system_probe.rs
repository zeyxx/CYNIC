//! SystemProbe concern: collect hardware metrics (CPU, RAM, Disk)
//! and POST them as observations to the kernel.

use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use tracing::{error, info, warn};

pub(crate) async fn run_system_probe(
    client: Client,
    kernel_url: String,
    api_key: String,
    node_name: String,
    interval_secs: u64,
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

        // Disk usage (sum of all disks)
        let mut disk_total = 0.0;
        let mut disk_available = 0.0;
        for disk in &disks {
            disk_total += disk.total_space() as f64 / 1024.0 / 1024.0 / 1024.0;
            disk_available += disk.available_space() as f64 / 1024.0 / 1024.0 / 1024.0;
        }

        // Load average (1m)
        let load = System::load_average().one;

        // Uptime
        let uptime = System::uptime();

        // Best-effort ZFS health check (Linux only)
        let mut zfs_health = serde_json::Value::Null;
        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("zpool")
                .args(["list", "-H", "-o", "name,health"])
                .output()
                && output.status.success()
            {
                let text = String::from_utf8_lossy(&output.stdout);
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
                zfs_health = json!(pools);
            }
        }

        let metrics = json!({
            "cpu_usage_percent": cpu_usage,
            "memory_used_gb": mem_used,
            "memory_total_gb": mem_total,
            "disk_total_gb": disk_total,
            "disk_available_gb": disk_available,
            "load_average_1m": load,
            "uptime_seconds": uptime,
            "zfs_pools": zfs_health,
        });

        let payload = json!({
            "tool": "SystemProbe",
            "domain": "infra",
            "status": "ok",
            "agent_id": node_name,
            "context": format!("System metrics for node {}", node_name),
            "value": metrics,
        });

        let url = format!("{}/observe", kernel_url.trim_end_matches('/'));

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
