//! Local CPU observation adapter.
//!
//! Reads `/proc/cpuinfo`, `/proc/stat`, and `/proc/pressure/cpu` directly from
//! the local kernel (no external host dependency) and assembles the 4-layer
//! `CpuReport` via the pure parsers in `domain::cpu::probe`.

use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::domain::cpu::port::{CpuObservationPort, CpuObservationError};
use crate::domain::cpu::{
    parse_cpuinfo, parse_pressure_cpu, parse_stat, CpuReport,
};

/// Local CPU adapter. Stateless — reads `/proc` on each call.
pub struct LocalCpuAdapter;

impl LocalCpuAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalCpuAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuObservationPort for LocalCpuAdapter {
    fn observe_cpu(&self) -> Result<CpuReport, CpuObservationError> {
        let mut residual_unknowns: Vec<String> = Vec::new();

        let stat_text = read_file_lossy("/proc/stat");
        let cpuinfo_text = read_file_lossy("/proc/cpuinfo");
        let pressure_text = read_file_lossy("/proc/pressure/cpu");

        let (physical, volume, mut unknowns) =
            parse_cpuinfo(cpuinfo_text.as_deref().unwrap_or(""));
        residual_unknowns.append(&mut unknowns);

        let usage = parse_stat(stat_text.as_deref().unwrap_or(""));
        if usage.is_none() {
            residual_unknowns.push("proc_stat_not_observed".to_string());
        }

        let pressure = if let Some(t) = &pressure_text {
            parse_pressure_cpu(t)
        } else {
            residual_unknowns.push("proc_pressure_cpu_missing".to_string());
            None
        };
        if pressure.is_some() && pressure.as_ref().map(|p| p.some_avg10).is_none() {
            residual_unknowns.push("cpu_psi_some_missing".to_string());
        }

        let mut report = CpuReport {
            schema: "cynic.host_cpu_report.v1".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            physical,
            volume,
            usage: usage.unwrap_or_else(default_usage),
            pressure,
            residual_unknowns,
        };

        // Fill governor from sysfs if present (level 2).
        if let Some(gov) = read_file_lossy("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
            .map(|s| s.trim().to_string())
        {
            report.volume.governor = Some(gov);
        }
        if let Some(iso) = read_file_lossy("/sys/devices/system/cpu/isolated")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            report.volume.isolated_mask = Some(iso);
        }

        Ok(report)
    }
}

fn read_file_lossy(path: &str) -> Option<String> {
    match fs::read_to_string(Path::new(path)) {
        Ok(s) => Some(s),
        Err(_) => None,
    }
}

fn default_usage() -> crate::domain::cpu::CpuUsage {
    use crate::domain::cpu::CpuUsage;
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
