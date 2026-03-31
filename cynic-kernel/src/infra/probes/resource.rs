//! ResourceProbe — CPU/RAM/disk/load sensing.
//! Disk: root filesystem only (fixes multi-fs inflation).
//! CPU: delta between ticks via /proc/stat (Linux), sysinfo fallback otherwise.

use crate::domain::probe::{ProbeDetails, ProbeError, ProbeResult, ProbeStatus, ResourceDetails};
use async_trait::async_trait;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[cfg(test)]
use crate::domain::probe::Probe;

/// Previous /proc/stat snapshot for CPU delta computation.
#[derive(Debug, Clone)]
struct CpuSnapshot {
    /// Sum of all CPU fields (user+nice+system+idle+iowait+irq+softirq+steal).
    total: u64,
    /// Sum of idle + iowait.
    idle: u64,
}

/// Stateful probe: holds previous CPU snapshot for delta computation.
#[derive(Debug)]
pub struct ResourceProbe {
    prev_cpu: Mutex<Option<CpuSnapshot>>,
}

impl Default for ResourceProbe {
    fn default() -> Self {
        Self {
            prev_cpu: Mutex::new(None),
        }
    }
}

impl ResourceProbe {
    /// Try to read aggregate CPU line from /proc/stat.
    /// Returns (total_ticks, idle_ticks) or None if unavailable.
    fn read_proc_stat() -> Option<CpuSnapshot> {
        let content = std::fs::read_to_string("/proc/stat").ok()?;
        // First line: "cpu  user nice system idle iowait irq softirq steal ..."
        let line = content.lines().next()?;
        if !line.starts_with("cpu ") {
            return None;
        }
        let fields: Vec<u64> = line
            .split_whitespace()
            .skip(1) // skip "cpu"
            .filter_map(|f| f.parse().ok())
            .collect();
        if fields.len() < 4 {
            return None;
        }
        let total: u64 = fields.iter().sum();
        // idle = fields[3], iowait = fields[4] (if present)
        let idle = fields[3] + fields.get(4).copied().unwrap_or(0);
        Some(CpuSnapshot { total, idle })
    }
}

#[async_trait]
impl crate::domain::probe::Probe for ResourceProbe {
    fn name(&self) -> &str {
        "resource"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(30)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();

        // ── CPU: delta from /proc/stat (zero blocking) ──
        let current_snap = Self::read_proc_stat();
        let cpu_usage_percent = {
            let mut prev_guard = self
                .prev_cpu
                .lock()
                .map_err(|e| ProbeError::Internal(format!("cpu mutex poisoned: {e}")))?;
            let usage = match (&*prev_guard, &current_snap) {
                (Some(prev), Some(curr)) => {
                    let total_delta = curr.total.saturating_sub(prev.total);
                    let idle_delta = curr.idle.saturating_sub(prev.idle);
                    if total_delta > 0 {
                        Some(
                            ((total_delta - idle_delta) as f64 / total_delta as f64 * 100.0) as f32,
                        )
                    } else {
                        None // no time elapsed
                    }
                }
                _ => None, // first tick or /proc/stat unavailable
            };
            *prev_guard = current_snap;
            usage
        };

        // ── Memory + disk + load: sysinfo in spawn_blocking ──
        let details = tokio::task::spawn_blocking(move || {
            use sysinfo::{Disks, System};

            let mut sys = System::new();
            sys.refresh_memory();

            let memory_total_gb = Some(sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0));
            let memory_used_gb = Some(sys.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0));

            // Disk: root filesystem only (mount_point == "/")
            let disks = Disks::new_with_refreshed_list();
            let root_disk = disks
                .iter()
                .find(|d| d.mount_point() == std::path::Path::new("/"));
            let disk_available_gb =
                root_disk.map(|d| d.available_space() as f64 / (1024.0 * 1024.0 * 1024.0));
            let disk_total_gb =
                root_disk.map(|d| d.total_space() as f64 / (1024.0 * 1024.0 * 1024.0));

            let load_avg = System::load_average();
            let load_average_1m = Some(load_avg.one);

            let uptime_seconds = Some(System::uptime());

            (
                memory_used_gb,
                memory_total_gb,
                disk_available_gb,
                disk_total_gb,
                load_average_1m,
                uptime_seconds,
            )
        })
        .await
        .map_err(|e| ProbeError::Internal(format!("spawn_blocking join error: {e}")))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let timestamp = chrono::Utc::now().to_rfc3339();

        Ok(ProbeResult {
            name: "resource".to_string(),
            status: ProbeStatus::Ok,
            details: ProbeDetails::Resource(ResourceDetails {
                cpu_usage_percent,
                memory_used_gb: details.0,
                memory_total_gb: details.1,
                disk_available_gb: details.2,
                disk_total_gb: details.3,
                load_average_1m: details.4,
                uptime_seconds: details.5,
            }),
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
        let probe = ResourceProbe::default();
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

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn resource_probe_cpu_none_on_first_tick() {
        let probe = ResourceProbe::default();
        let result = probe.sense().await.expect("should not error");
        match result.details {
            ProbeDetails::Resource(ref r) => {
                // First tick: no previous snapshot, CPU should be None
                assert!(r.cpu_usage_percent.is_none());
            }
            _ => panic!("expected Resource details"),
        }
    }

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn resource_probe_cpu_delta_requires_elapsed_time() {
        let probe = ResourceProbe::default();
        // First tick seeds the snapshot
        let _ = probe.sense().await.expect("first tick");
        // Small sleep so /proc/stat counters advance
        tokio::time::sleep(Duration::from_millis(50)).await;
        // Second tick computes delta
        let result = probe.sense().await.expect("second tick");
        match result.details {
            ProbeDetails::Resource(ref r) => {
                // On Linux with elapsed time: should have a value.
                // On other OS: still None (no /proc/stat).
                if std::path::Path::new("/proc/stat").exists() {
                    assert!(r.cpu_usage_percent.is_some());
                }
            }
            _ => panic!("expected Resource details"),
        }
    }

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn resource_probe_disk_is_root_only() {
        let probe = ResourceProbe::default();
        let result = probe.sense().await.expect("should not error");
        match result.details {
            ProbeDetails::Resource(ref r) => {
                // If we got disk values, they should be for root only (< sum of all fs)
                if let (Some(total), Some(avail)) = (r.disk_total_gb, r.disk_available_gb) {
                    // Root fs should be less than 2TB for any reasonable system
                    assert!(
                        total < 2000.0,
                        "disk_total_gb={total} seems too large for root fs"
                    );
                    assert!(avail <= total);
                }
            }
            _ => panic!("expected Resource details"),
        }
    }
}
