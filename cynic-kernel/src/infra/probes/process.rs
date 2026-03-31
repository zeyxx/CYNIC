//! ProcessProbe — self-monitoring via /proc/self.
//! Reads RSS, fd count/limit, thread count, and cumulative CPU time.
//! On non-Linux: returns ProbeStatus::Unavailable (no #[cfg] in domain — K1 compliant).

use crate::domain::probe::{ProbeDetails, ProbeError, ProbeResult, ProbeStatus, ProcessDetails};
use async_trait::async_trait;
use std::time::{Duration, Instant};

#[cfg(test)]
use crate::domain::probe::Probe;

/// Probe that reads /proc/self/* for kernel self-monitoring.
#[derive(Debug, Default)]
pub struct ProcessProbe;

impl ProcessProbe {
    /// Read VmRSS from /proc/self/status (in kB → MB).
    fn read_rss_mb() -> Option<f64> {
        let content = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                let kb: f64 = rest.split_whitespace().next()?.parse().ok()?;
                return Some(kb / 1024.0);
            }
        }
        None
    }

    /// Read thread count from /proc/self/status.
    fn read_thread_count() -> Option<u64> {
        let content = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("Threads:") {
                return rest.trim().parse().ok();
            }
        }
        None
    }

    /// Count entries in /proc/self/fd.
    fn read_fd_count() -> Option<u64> {
        let entries = std::fs::read_dir("/proc/self/fd").ok()?;
        Some(entries.count() as u64)
    }

    /// Read soft RLIMIT_NOFILE from /proc/self/limits.
    fn read_fd_limit() -> Option<u64> {
        let content = std::fs::read_to_string("/proc/self/limits").ok()?;
        for line in content.lines() {
            if line.starts_with("Max open files") {
                // Format: "Max open files            1048576              1048576              files"
                let fields: Vec<&str> = line.split_whitespace().collect();
                // Soft limit is field after "files" label (index 3)
                return fields.get(3)?.parse().ok();
            }
        }
        None
    }

    /// Read utime and stime from /proc/self/stat (fields 14 and 15, 1-indexed).
    /// Returns (user_seconds, system_seconds).
    fn read_cpu_times() -> Option<(f64, f64)> {
        let content = std::fs::read_to_string("/proc/self/stat").ok()?;
        // Field layout: pid (comm) state ppid ... utime(14) stime(15)
        // comm can contain spaces/parens, so find the closing ')' first.
        let after_comm = content.split(')').nth(1)?;
        let fields: Vec<&str> = after_comm.split_whitespace().collect();
        // After ')': state(0) ppid(1) pgrp(2) session(3) tty_nr(4) tpgid(5)
        //            flags(6) minflt(7) cminflt(8) majflt(9) cmajflt(10)
        //            utime(11) stime(12)
        let clk_tck = 100.0_f64; // standard Linux USER_HZ
        let utime: f64 = fields.get(11)?.parse().ok()?;
        let stime: f64 = fields.get(12)?.parse().ok()?;
        Some((utime / clk_tck, stime / clk_tck))
    }
}

#[async_trait]
impl crate::domain::probe::Probe for ProcessProbe {
    fn name(&self) -> &str {
        "process"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(30)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();

        // All /proc reads are microseconds — no need for spawn_blocking.
        let memory_rss_mb = Self::read_rss_mb();
        let fd_count = Self::read_fd_count();
        let fd_limit = Self::read_fd_limit();
        let thread_count = Self::read_thread_count();
        let (cpu_user_seconds, cpu_system_seconds) = Self::read_cpu_times()
            .map(|(u, s)| (Some(u), Some(s)))
            .unwrap_or((None, None));

        let duration_ms = start.elapsed().as_millis() as u64;
        let timestamp = chrono::Utc::now().to_rfc3339();

        // If we can't read anything, mark as unavailable (non-Linux graceful degradation).
        let status = if memory_rss_mb.is_none() && fd_count.is_none() && thread_count.is_none() {
            ProbeStatus::Unavailable
        } else {
            ProbeStatus::Ok
        };

        Ok(ProbeResult {
            name: "process".to_string(),
            status,
            details: ProbeDetails::Process(ProcessDetails {
                memory_rss_mb,
                fd_count,
                fd_limit,
                thread_count,
                cpu_user_seconds,
                cpu_system_seconds,
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
    async fn process_probe_returns_result() {
        let probe = ProcessProbe;
        let result = probe.sense().await.expect("should not error");
        if std::path::Path::new("/proc/self/status").exists() {
            assert_eq!(result.status, ProbeStatus::Ok);
            match result.details {
                ProbeDetails::Process(ref p) => {
                    assert!(p.memory_rss_mb.is_some());
                    assert!(p.memory_rss_mb.unwrap_or(0.0) > 0.0);
                    assert!(p.fd_count.is_some());
                    assert!(p.thread_count.is_some());
                }
                _ => panic!("expected Process details"),
            }
        } else {
            assert_eq!(result.status, ProbeStatus::Unavailable);
        }
    }

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn process_probe_cpu_times_positive() {
        if !std::path::Path::new("/proc/self/stat").exists() {
            return; // skip on non-Linux
        }
        let probe = ProcessProbe;
        let result = probe.sense().await.expect("should not error");
        match result.details {
            ProbeDetails::Process(ref p) => {
                // cpu_user_seconds should be non-negative
                if let Some(u) = p.cpu_user_seconds {
                    assert!(u >= 0.0);
                }
            }
            _ => panic!("expected Process details"),
        }
    }
}
