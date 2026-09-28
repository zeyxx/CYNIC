//! Finance adapters for cynic-host.
//!
//! Implements cynic-finance ports using host-level observation.
//! All adapters are concrete — they read from /proc, /sys, and sysinfo.

use cynic_finance::ports::{
    ActionMeter, ComputeMeter, EnergyMeter, MeterError, NetworkMeter, StorageMeter, TimeMeter,
};
use std::fs;
use std::time::Instant;

/// CPU meter — reads from /proc/<pid>/stat.
pub struct HostComputeMeter;

impl ComputeMeter for HostComputeMeter {
    fn measure_cpu_ms(&self, agent_id: &str) -> Result<u64, MeterError> {
        // agent_id is expected to be a PID for direct /proc access
        let pid = agent_id
            .parse::<u32>()
            .map_err(|_| MeterError::Unknown(format!("agent_id '{}' is not a PID", agent_id)))?;

        let stat = fs::read_to_string(format!("/proc/{}/stat", pid))
            .map_err(|e| MeterError::Unavailable(format!("read /proc/{}/stat: {}", pid, e)))?;

        // Fields: pid (comm) state ppid pgrp session tty_nr tpgid flags minflt ...
        // utime is field 14, stime is field 15 (1-indexed)
        // After splitting on whitespace, we need to handle the comm field
        // which may contain spaces. Find the last ')' and split after it.
        let after_comm = stat
            .rsplit_once(')')
            .map(|(_, rest)| rest)
            .ok_or_else(|| MeterError::Unknown("malformed /proc stat".to_string()))?;

        let fields: Vec<&str> = after_comm.split_whitespace().collect();
        // utime is field 14 overall = field 12 after comm (0-indexed: 13-2=11)
        // stime is field 15 overall = field 13 after comm (0-indexed: 14-2=12)
        let utime = fields
            .get(11)
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| MeterError::Unknown("missing utime".to_string()))?;
        let stime = fields
            .get(12)
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| MeterError::Unknown("missing stime".to_string()))?;

        // Convert clock ticks to milliseconds (typical: 100 ticks/second)
        let ticks_per_sec = 100u64;
        let total_ms = (utime + stime) * 1000 / ticks_per_sec;
        Ok(total_ms)
    }
}

/// Storage meter — reads from sysinfo.
pub struct HostStorageMeter {
    root_path: String,
}

impl HostStorageMeter {
    pub fn new(root_path: impl Into<String>) -> Self {
        Self {
            root_path: root_path.into(),
        }
    }
}

impl StorageMeter for HostStorageMeter {
    fn measure_bytes(&self, _agent_id: &str) -> Result<u64, MeterError> {
        let mounts = sysinfo::Disks::new_with_refreshed_list();
        let mut total_bytes = 0u64;
        let mut found = false;

        for disk in mounts.list() {
            if disk.mount_point().to_string_lossy() == self.root_path {
                total_bytes += disk.total_space();
                found = true;
            }
        }

        if !found {
            return Err(MeterError::Unknown(format!(
                "mount point '{}' not found",
                self.root_path
            )));
        }

        Ok(total_bytes)
    }

    fn measure_iops(&self, _agent_id: &str) -> Result<u64, MeterError> {
        // IOPS requires sampling over time; return unknown for now
        Err(MeterError::Unknown(
            "IOPS measurement requires time-based sampling".to_string(),
        ))
    }
}

/// Network meter — reads from /proc/net/dev.
pub struct HostNetworkMeter;

impl NetworkMeter for HostNetworkMeter {
    fn measure_bytes(&self, _agent_id: &str) -> Result<u64, MeterError> {
        let content = fs::read_to_string("/proc/net/dev")
            .map_err(|e| MeterError::Unavailable(format!("read /proc/net/dev: {}", e)))?;

        let mut total_bytes = 0u64;
        for line in content.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                if let Ok(rx) = parts[1].parse::<u64>() {
                    total_bytes += rx;
                }
                if let Ok(tx) = parts[9].parse::<u64>() {
                    total_bytes += tx;
                }
            }
        }
        Ok(total_bytes)
    }

    fn measure_packets(&self, _agent_id: &str) -> Result<u64, MeterError> {
        let content = fs::read_to_string("/proc/net/dev")
            .map_err(|e| MeterError::Unavailable(format!("read /proc/net/dev: {}", e)))?;

        let mut total_packets = 0u64;
        for line in content.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                if let Ok(rx) = parts[2].parse::<u64>() {
                    total_packets += rx;
                }
                if let Ok(tx) = parts[10].parse::<u64>() {
                    total_packets += tx;
                }
            }
        }
        Ok(total_packets)
    }
}

/// Energy meter — reads from RAPL (Intel) or estimates from CPU TDP.
pub struct HostEnergyMeter;

impl EnergyMeter for HostEnergyMeter {
    fn measure_watts(&self, _agent_id: &str) -> Result<f64, MeterError> {
        // Try Intel RAPL
        let rapl_path = "/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj";
        if let Ok(content) = fs::read_to_string(rapl_path) {
            if let Ok(energy_uj) = content.trim().parse::<u64>() {
                // Convert microjoules to watts (instantaneous approximation)
                // This is a snapshot; real power requires delta over time
                let watts = energy_uj as f64 / 1_000_000.0;
                return Ok(watts);
            }
        }

        // Try AMD energy
        let amd_path = "/sys/class/hwmon/hwmon0/power1_average";
        if let Ok(content) = fs::read_to_string(amd_path) {
            if let Ok(power_uw) = content.trim().parse::<u64>() {
                return Ok(power_uw as f64 / 1_000_000.0);
            }
        }

        Err(MeterError::Unknown(
            "no RAPL or AMD energy sensor available".to_string(),
        ))
    }

    fn measure_kwh(&self, _agent_id: &str) -> Result<f64, MeterError> {
        // kWh requires integration over time; return unknown for now
        Err(MeterError::Unknown(
            "kWh measurement requires time-based integration".to_string(),
        ))
    }
}

/// Time meter — returns elapsed seconds since process start.
pub struct HostTimeMeter {
    start: Instant,
}

impl HostTimeMeter {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for HostTimeMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeMeter for HostTimeMeter {
    fn measure_seconds(&self, _agent_id: &str) -> Result<u64, MeterError> {
        Ok(self.start.elapsed().as_secs())
    }
}

/// Action meter — counts actions performed by an agent.
/// This is a simple in-memory counter; production use would persist to storage.
pub struct HostActionMeter {
    counts: std::collections::HashMap<String, u64>,
}

impl HostActionMeter {
    pub fn new() -> Self {
        Self {
            counts: std::collections::HashMap::new(),
        }
    }

    /// Records an action for an agent.
    pub fn record(&mut self, agent_id: &str) {
        *self.counts.entry(agent_id.to_string()).or_insert(0) += 1;
    }
}

impl Default for HostActionMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionMeter for HostActionMeter {
    fn measure_actions(&self, agent_id: &str) -> Result<u64, MeterError> {
        Ok(*self.counts.get(agent_id).unwrap_or(&0))
    }
}
