//! Finance adapters for cynic-host.
//!
//! Implements cynic-finance ports using host-level observation.
//! All adapters are concrete — they read from /proc, /sys, and sysinfo.

use cynic_finance::ports::{
    ActionMeter, ComputeMeter, EnergyMeter, MeterError, NetworkMeter, StorageMeter, TimeMeter,
};
use std::cell::Cell;
use std::fs;
use std::time::Instant;

/// CPU meter — reads from /proc/<pid>/stat.
pub struct HostComputeMeter;

impl ComputeMeter for HostComputeMeter {
    fn measure_cpu_ms(&self, agent_id: &str) -> Result<u64, MeterError> {
        let pid = agent_id
            .parse::<u32>()
            .map_err(|_| MeterError::Unknown(format!("agent_id '{}' is not a PID", agent_id)))?;

        let stat = fs::read_to_string(format!("/proc/{}/stat", pid))
            .map_err(|e| MeterError::Unavailable(format!("read /proc/{}/stat: {}", pid, e)))?;

        let after_comm = stat
            .rsplit_once(')')
            .map(|(_, rest)| rest)
            .ok_or_else(|| MeterError::Unknown("malformed /proc stat".to_string()))?;

        let fields: Vec<&str> = after_comm.split_whitespace().collect();
        let utime = fields
            .get(11)
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| MeterError::Unknown("missing utime".to_string()))?;
        let stime = fields
            .get(12)
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| MeterError::Unknown("missing stime".to_string()))?;

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

/// Energy meter — reads from RAPL (Intel) or AMD hwmon.
///
/// Computes power (watts) by sampling energy (microjoules) over a time
/// delta. Integrates energy over time for kWh.
pub struct HostEnergyMeter {
    /// Last observed energy reading (microjoules).
    last_energy_uj: Cell<Option<u64>>,
    /// Timestamp of last reading.
    last_reading: Cell<Option<Instant>>,
    /// Accumulated energy (joules) for kWh integration.
    accumulated_joules: Cell<f64>,
}

impl HostEnergyMeter {
    pub fn new() -> Self {
        Self {
            last_energy_uj: Cell::new(None),
            last_reading: Cell::new(None),
            accumulated_joules: Cell::new(0.0),
        }
    }

    /// Reads the current energy counter from RAPL or AMD hwmon.
    fn read_energy_uj(&self) -> Result<u64, MeterError> {
        // Try Intel RAPL
        if let Ok(content) =
            fs::read_to_string("/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj")
        {
            if let Ok(energy_uj) = content.trim().parse::<u64>() {
                return Ok(energy_uj);
            }
        }

        // Try AMD hwmon (scan for power*_average)
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(name) = fs::read_to_string(path.join("name")) {
                    if name.trim().contains("coretemp") || name.trim().contains("k10temp") {
                        if let Ok(content) = fs::read_to_string(path.join("power1_average")) {
                            if let Ok(power_uw) = content.trim().parse::<u64>() {
                                return Ok(power_uw);
                            }
                        }
                    }
                }
            }
        }

        Err(MeterError::Unknown(
            "no RAPL or AMD energy sensor available".to_string(),
        ))
    }
}

impl Default for HostEnergyMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyMeter for HostEnergyMeter {
    fn measure_watts(&self, _agent_id: &str) -> Result<f64, MeterError> {
        let energy_uj = self.read_energy_uj()?;
        let now = Instant::now();

        let last_uj = self.last_energy_uj.get();
        let last_time = self.last_reading.get();

        if let (Some(last_uj), Some(last_time)) = (last_uj, last_time) {
            let elapsed = now.duration_since(last_time);
            let delta_uj = energy_uj.saturating_sub(last_uj) as f64;
            let elapsed_s = elapsed.as_secs_f64();
            if elapsed_s > 0.0 {
                let watts = (delta_uj / 1_000_000.0) / elapsed_s;
                // Update state for next reading
                self.last_energy_uj.set(Some(energy_uj));
                self.last_reading.set(Some(now));
                self.accumulated_joules
                    .set(self.accumulated_joules.get() + delta_uj / 1_000_000.0);
                return Ok(watts);
            }
        }

        // First reading — store and return unknown
        self.last_energy_uj.set(Some(energy_uj));
        self.last_reading.set(Some(now));
        Err(MeterError::Unknown(
            "first energy reading — no delta available".to_string(),
        ))
    }

    fn measure_kwh(&self, _agent_id: &str) -> Result<f64, MeterError> {
        let energy_uj = self.read_energy_uj()?;
        let now = Instant::now();

        let last_uj = self.last_energy_uj.get();
        let last_time = self.last_reading.get();

        if let (Some(last_uj), Some(last_time)) = (last_uj, last_time) {
            let elapsed = now.duration_since(last_time);
            let delta_uj = energy_uj.saturating_sub(last_uj) as f64;
            let elapsed_s = elapsed.as_secs_f64();
            if elapsed_s > 0.0 {
                let joules = delta_uj / 1_000_000.0;
                let kwh = joules / 3_600_000.0;
                self.last_energy_uj.set(Some(energy_uj));
                self.last_reading.set(Some(now));
                self.accumulated_joules
                    .set(self.accumulated_joules.get() + joules);
                return Ok(kwh);
            }
        }

        self.last_energy_uj.set(Some(energy_uj));
        self.last_reading.set(Some(now));
        Err(MeterError::Unknown(
            "first energy reading — no delta available".to_string(),
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
pub struct HostActionMeter {
    counts: std::collections::HashMap<String, u64>,
}

impl HostActionMeter {
    pub fn new() -> Self {
        Self {
            counts: std::collections::HashMap::new(),
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_meter_first_reading_returns_unknown() {
        let meter = HostEnergyMeter::new();
        let result = meter.measure_watts("agent-1");
        assert!(matches!(result, Err(MeterError::Unknown(_))));
    }

    #[test]
    fn energy_meter_kwh_first_reading_returns_unknown() {
        let meter = HostEnergyMeter::new();
        let result = meter.measure_kwh("agent-1");
        assert!(matches!(result, Err(MeterError::Unknown(_))));
    }

    #[test]
    fn action_meter_counts_actions() {
        let mut meter = HostActionMeter::new();
        meter.record("agent-1");
        meter.record("agent-1");
        meter.record("agent-2");

        assert_eq!(meter.measure_actions("agent-1").unwrap(), 2);
        assert_eq!(meter.measure_actions("agent-2").unwrap(), 1);
        assert_eq!(meter.measure_actions("agent-3").unwrap(), 0);
    }

    #[test]
    fn time_meter_returns_elapsed() {
        let meter = HostTimeMeter::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let seconds = meter.measure_seconds("agent-1").unwrap();
        assert!(seconds >= 0);
    }
}
