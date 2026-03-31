//! PressureProbe — Linux PSI (Pressure Stall Information) from /proc/pressure/*.
//! On non-Linux: returns ProbeStatus::Unavailable (no #[cfg] in domain — K1 compliant).

use crate::domain::probe::{PressureDetails, ProbeDetails, ProbeError, ProbeResult, ProbeStatus};
use async_trait::async_trait;
use std::time::{Duration, Instant};

#[cfg(test)]
use crate::domain::probe::Probe;

/// Probe that reads /proc/pressure/{cpu,memory,io} for system pressure signals.
#[derive(Debug, Default)]
pub struct PressureProbe;

impl PressureProbe {
    /// Parse a PSI file, returning (some_avg10, some_avg60, full_avg10).
    /// PSI format: "some avg10=X.XX avg60=X.XX avg300=X.XX total=N"
    ///             "full avg10=X.XX avg60=X.XX avg300=X.XX total=N"  (not for cpu)
    fn parse_psi(content: &str) -> (Option<f64>, Option<f64>, Option<f64>) {
        let mut some_avg10 = None;
        let mut some_avg60 = None;
        let mut full_avg10 = None;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            let is_some = parts[0] == "some";
            let is_full = parts[0] == "full";
            if !is_some && !is_full {
                continue;
            }
            for part in &parts[1..] {
                if let Some(val_str) = part.strip_prefix("avg10=") {
                    if let Ok(v) = val_str.parse::<f64>() {
                        if is_some {
                            some_avg10 = Some(v);
                        } else {
                            full_avg10 = Some(v);
                        }
                    }
                } else if let Some(val_str) = part.strip_prefix("avg60=")
                    && let Ok(v) = val_str.parse::<f64>()
                    && is_some
                {
                    some_avg60 = Some(v);
                }
            }
        }
        (some_avg10, some_avg60, full_avg10)
    }
}

#[async_trait]
impl crate::domain::probe::Probe for PressureProbe {
    fn name(&self) -> &str {
        "pressure"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(30)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();

        let cpu_psi = std::fs::read_to_string("/proc/pressure/cpu").ok();
        let mem_psi = std::fs::read_to_string("/proc/pressure/memory").ok();
        let io_psi = std::fs::read_to_string("/proc/pressure/io").ok();

        let duration_ms = start.elapsed().as_millis() as u64;
        let timestamp = chrono::Utc::now().to_rfc3339();

        // If none of the PSI files are readable, mark as unavailable.
        if cpu_psi.is_none() && mem_psi.is_none() && io_psi.is_none() {
            return Ok(ProbeResult {
                name: "pressure".to_string(),
                status: ProbeStatus::Unavailable,
                details: ProbeDetails::Pressure(PressureDetails {
                    cpu_some_avg10: None,
                    cpu_some_avg60: None,
                    memory_some_avg10: None,
                    memory_full_avg10: None,
                    io_some_avg10: None,
                    io_full_avg10: None,
                }),
                duration_ms,
                timestamp,
            });
        }

        let (cpu_some_avg10, cpu_some_avg60, _) = cpu_psi
            .as_deref()
            .map(Self::parse_psi)
            .unwrap_or((None, None, None));
        let (memory_some_avg10, _, memory_full_avg10) = mem_psi
            .as_deref()
            .map(Self::parse_psi)
            .unwrap_or((None, None, None));
        let (io_some_avg10, _, io_full_avg10) = io_psi
            .as_deref()
            .map(Self::parse_psi)
            .unwrap_or((None, None, None));

        Ok(ProbeResult {
            name: "pressure".to_string(),
            status: ProbeStatus::Ok,
            details: ProbeDetails::Pressure(PressureDetails {
                cpu_some_avg10,
                cpu_some_avg60,
                memory_some_avg10,
                memory_full_avg10,
                io_some_avg10,
                io_full_avg10,
            }),
            duration_ms,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_psi_cpu() {
        let input = "some avg10=1.23 avg60=4.56 avg300=7.89 total=123456\n";
        let (some10, some60, full10) = PressureProbe::parse_psi(input);
        assert!((some10.unwrap() - 1.23).abs() < 0.001);
        assert!((some60.unwrap() - 4.56).abs() < 0.001);
        assert!(full10.is_none()); // cpu has no "full" line
    }

    #[test]
    fn parse_psi_memory() {
        let input = "some avg10=0.50 avg60=1.20 avg300=2.30 total=100\nfull avg10=0.10 avg60=0.30 avg300=0.50 total=50\n";
        let (some10, some60, full10) = PressureProbe::parse_psi(input);
        assert!((some10.unwrap() - 0.50).abs() < 0.001);
        assert!((some60.unwrap() - 1.20).abs() < 0.001);
        assert!((full10.unwrap() - 0.10).abs() < 0.001);
    }

    #[test]
    fn parse_psi_empty() {
        let (some10, some60, full10) = PressureProbe::parse_psi("");
        assert!(some10.is_none());
        assert!(some60.is_none());
        assert!(full10.is_none());
    }

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn pressure_probe_returns_result() {
        let probe = PressureProbe;
        let result = probe.sense().await.expect("should not error");
        if std::path::Path::new("/proc/pressure/cpu").exists() {
            assert_eq!(result.status, ProbeStatus::Ok);
            match result.details {
                ProbeDetails::Pressure(ref p) => {
                    assert!(p.cpu_some_avg10.is_some());
                }
                _ => panic!("expected Pressure details"),
            }
        } else {
            assert_eq!(result.status, ProbeStatus::Unavailable);
        }
    }
}
