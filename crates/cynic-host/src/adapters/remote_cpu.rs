//! Remote CPU observation adapter (SSH).
//!
//! Drives the same pure parsers as `LocalCpuAdapter` but over a remote host via
//! an injected `ArgvRunner` (so the adapter is unit-testable with `MapArgvRunner`
//! instead of a live SSH).

use std::sync::Arc;

use chrono::Utc;

use crate::domain::cpu::port::{CpuObservationError, CpuObservationPort};
use crate::domain::cpu::{CpuReport, CpuUsage, parse_cpuinfo, parse_pressure_cpu, parse_stat};
use crate::update_lifecycle::ArgvRunner;

/// SSH remote CPU adapter. Generic over the argv runner for testability.
pub struct RemoteCpuAdapter<R: ArgvRunner> {
    runner: Arc<R>,
    /// Rendered SSH target, e.g. `cynic@10.0.0.7`.
    ssh_target: String,
}

impl<R: ArgvRunner> RemoteCpuAdapter<R> {
    /// Build a remote adapter. `ssh_target` is the full SSH destination.
    pub fn new(runner: R, ssh_target: String) -> Result<Self, String> {
        if ssh_target.is_empty() {
            return Err("empty_ssh_target".to_string());
        }
        Ok(Self {
            runner: Arc::new(runner),
            ssh_target,
        })
    }

    fn run_ssh(&self, script: &str) -> Result<String, CpuObservationError> {
        let argv = ["ssh", &self.ssh_target, script];
        let out = self.runner.run(&argv);
        if out.success() {
            Ok(out.stdout)
        } else {
            Err(format!(
                "remote_cpu_failed:{}",
                out.spawn_error.clone().unwrap_or_default()
            ))
        }
    }
}

impl<R: ArgvRunner> CpuObservationPort for RemoteCpuAdapter<R> {
    fn observe_cpu(&self) -> Result<CpuReport, CpuObservationError> {
        let mut residual_unknowns: Vec<String> = Vec::new();

        let cpuinfo = self.run_ssh("cat /proc/cpuinfo")?;
        let stat_text = self.run_ssh("cat /proc/stat")?;

        // pressure is best-effort (older kernels lack it).
        let pressure = match self.run_ssh("cat /proc/pressure/cpu") {
            Ok(t) => parse_pressure_cpu(&t),
            Err(_) => {
                residual_unknowns.push("cpu_psi_missing_remote".to_string());
                None
            }
        };
        if pressure.as_ref().and_then(|p| p.some_avg10).is_none() {
            residual_unknowns.push("cpu_psi_some_missing_remote".to_string());
        }

        let (physical, volume, mut unknowns) = parse_cpuinfo(&cpuinfo);
        residual_unknowns.append(&mut unknowns);

        let usage = parse_stat(&stat_text);
        if usage.is_none() {
            residual_unknowns.push("proc_stat_not_observed".to_string());
        }

        let timestamp = Utc::now().to_rfc3339();
        Ok(CpuReport {
            schema: "cynic.host_cpu_report.v1".to_string(),
            timestamp,
            physical,
            volume,
            usage: usage.unwrap_or_else(default_usage),
            pressure,
            residual_unknowns,
        })
    }
}

fn default_usage() -> CpuUsage {
    CpuUsage {
        user: 0,
        system: 0,
        idle: 0,
        iowait: 0,
        softirq: 0,
        steal: 0,
        util_pct: None,
    }
}
