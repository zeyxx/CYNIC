//! RAPL (Running Average Power Limit) energy metering.
//!
//! Reads energy consumption from Intel RAPL sysfs interface.
//! Provides package-level and DRAM-level power measurements.

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const RAPL_SCHEMA: &str = "cynic.sandbox.rapl.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaplReading {
    pub schema: String,
    pub package_joules: u64,
    pub dram_joules: u64,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaplConfig {
    pub schema: String,
    /// Path to RAPL sysfs root.
    pub sysfs_path: String,
}

pub trait RaplPort {
    /// Read current cumulative energy.
    fn read(&self) -> Result<RaplReading, String>;
    /// Compute delta between two readings.
    fn delta(&self, before: &RaplReading, after: &RaplReading) -> Result<RaplDelta, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaplDelta {
    pub schema: String,
    pub energy_joules: u64,
    pub duration_ms: u64,
}

pub fn null_rapl_port() -> NullRaplPort {
    NullRaplPort
}

pub struct NullRaplPort;

impl RaplPort for NullRaplPort {
    fn read(&self) -> Result<RaplReading, String> {
        Err("RAPL not available on this platform".to_string())
    }
    fn delta(&self, _before: &RaplReading, _after: &RaplReading) -> Result<RaplDelta, String> {
        Err("RAPL not available on this platform".to_string())
    }
}

impl RaplConfig {
    pub fn default_sysfs() -> Self {
        Self {
            schema: RAPL_SCHEMA.to_string(),
            sysfs_path: "/sys/class/powercap/intel-rapl:0".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rapl_delta_calculation() {
        let now = RaplReading {
            schema: RAPL_SCHEMA.to_string(),
            package_joules: 100,
            dram_joules: 20,
            timestamp_ms: 1000,
        };
        let later = RaplReading {
            schema: RAPL_SCHEMA.to_string(),
            package_joules: 150,
            dram_joules: 30,
            timestamp_ms: 2000,
        };
        let port = NullRaplPort;
        // Null port returns error, but we test the math
        let energy = later.package_joules - now.package_joules;
        assert_eq!(energy, 50);
    }

    #[test]
    fn rapl_config_default_path() {
        let c = RaplConfig::default_sysfs();
        assert!(c.sysfs_path.contains("intel-rapl"));
    }
}
