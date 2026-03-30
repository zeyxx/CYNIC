//! ResourceProbe — CPU/RAM/disk/load sensing via sysinfo.
//! Follows the same spawn_blocking + two-pass CPU pattern as SysinfoMetrics.

use crate::domain::probe::{ProbeDetails, ProbeError, ProbeResult, ProbeStatus, ResourceDetails};
use async_trait::async_trait;
use std::time::{Duration, Instant};

#[cfg(test)]
use crate::domain::probe::Probe;

/// Stateless probe that reads CPU, memory, disk, and load from the host OS.
#[derive(Debug, Default)]
pub struct ResourceProbe;

#[async_trait]
impl crate::domain::probe::Probe for ResourceProbe {
    fn name(&self) -> &str {
        "resource"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(300)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();

        let details = tokio::task::spawn_blocking(|| {
            use sysinfo::{Disks, System};

            let mut sys = System::new();
            sys.refresh_memory();
            sys.refresh_cpu_usage();

            // CPU usage requires two samples with a gap between them.
            // std::thread::sleep is safe here — we are inside spawn_blocking.
            std::thread::sleep(Duration::from_millis(200));
            sys.refresh_cpu_usage();

            let cpu_usage_percent = if sys.cpus().is_empty() {
                None
            } else {
                let avg =
                    sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32;
                Some(avg)
            };

            let memory_total_gb = Some(sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0));
            let memory_used_gb = Some(sys.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0));

            let disks = Disks::new_with_refreshed_list();
            let (disk_available, disk_total) = disks.iter().fold((0u64, 0u64), |(a, t), d| {
                (a + d.available_space(), t + d.total_space())
            });
            let disk_available_gb = Some(disk_available as f64 / (1024.0 * 1024.0 * 1024.0));
            let disk_total_gb = Some(disk_total as f64 / (1024.0 * 1024.0 * 1024.0));

            let load_avg = System::load_average();
            let load_average_1m = Some(load_avg.one);

            let uptime_seconds = Some(System::uptime());

            ResourceDetails {
                cpu_usage_percent,
                memory_used_gb,
                memory_total_gb,
                disk_available_gb,
                disk_total_gb,
                load_average_1m,
                uptime_seconds,
            }
        })
        .await
        .map_err(|e| ProbeError::Internal(format!("spawn_blocking join error: {e}")))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let timestamp = chrono::Utc::now().to_rfc3339();

        Ok(ProbeResult {
            name: "resource".to_string(),
            status: ProbeStatus::Ok,
            details: ProbeDetails::Resource(details),
            duration_ms,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn resource_probe_returns_ok() {
        let probe = ResourceProbe;
        let result = probe.sense().await.expect("should not error");
        assert_eq!(result.status, ProbeStatus::Ok);
        match result.details {
            ProbeDetails::Resource(ref r) => {
                assert!(r.memory_total_gb.is_some());
                assert!(r.memory_total_gb.unwrap_or(0.0) > 0.0);
            }
            _ => panic!("expected Resource details"),
        }
    }
}
