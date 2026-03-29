//! sysinfo-based adapter for SystemMetricsPort.
//! Per-tick System::new() + targeted refresh — same proven pattern as probe/hardware.rs.
//! All sysinfo calls run inside spawn_blocking (they are synchronous).

use crate::domain::system_metrics::{SystemMetricsError, SystemMetricsPort, SystemSnapshot};
use async_trait::async_trait;

#[derive(Debug, Default)]
pub struct SysinfoMetrics;

impl SysinfoMetrics {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SystemMetricsPort for SysinfoMetrics {
    async fn snapshot(&self) -> Result<SystemSnapshot, SystemMetricsError> {
        tokio::task::spawn_blocking(|| {
            use sysinfo::{Disks, System};

            let mut sys = System::new();
            sys.refresh_memory();
            sys.refresh_cpu_usage();

            // CPU usage needs two samples. Brief sleep inside spawn_blocking
            // (blocking thread, not async — sleep is safe here).
            std::thread::sleep(std::time::Duration::from_millis(200));
            sys.refresh_cpu_usage();

            let cpu_usage = if sys.cpus().is_empty() {
                0.0
            } else {
                sys.cpus().iter().map(|c| c.cpu_usage() as f64).sum::<f64>()
                    / sys.cpus().len() as f64
            };

            let memory_total_gb = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
            let memory_used_gb = sys.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0);

            let disks = Disks::new_with_refreshed_list();
            let (disk_available, disk_total) = disks.iter().fold((0u64, 0u64), |(a, t), d| {
                (a + d.available_space(), t + d.total_space())
            });
            let disk_available_gb = disk_available as f64 / (1024.0 * 1024.0 * 1024.0);
            let disk_total_gb = disk_total as f64 / (1024.0 * 1024.0 * 1024.0);

            let load_avg = System::load_average();

            Ok(SystemSnapshot {
                cpu_usage_percent: cpu_usage,
                memory_used_gb,
                memory_total_gb,
                disk_available_gb,
                disk_total_gb,
                load_average_1m: load_avg.one,
                uptime_seconds: System::uptime(),
                created_at: chrono::Utc::now().to_rfc3339(),
            })
        })
        .await
        .map_err(|e| SystemMetricsError(format!("spawn_blocking join error: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn snapshot_returns_sane_values() {
        let metrics = SysinfoMetrics::new();
        let snap = metrics.snapshot().await.expect("snapshot should succeed");
        assert!(snap.memory_total_gb > 0.0, "total RAM must be positive");
        assert!(snap.memory_used_gb >= 0.0, "used RAM must be non-negative");
        assert!(snap.memory_used_gb <= snap.memory_total_gb, "used <= total");
        assert!(snap.disk_total_gb > 0.0, "disk total must be positive");
        assert!(snap.uptime_seconds > 0, "uptime must be positive");
        assert!(snap.cpu_usage_percent >= 0.0 && snap.cpu_usage_percent <= 100.0);
    }
}
