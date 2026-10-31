//! Finance adapters for cynic-network.
//!
//! Implements cynic-finance NetworkMeter port using /proc/net/dev and WireGuard telemetry.

use cynic_protocol::resource_meter_port::{MeterError, NetworkMeter};
use std::fs;

/// Network meter — measures network bytes and packets from /proc/net/dev.
pub struct NetworkFinanceMeter;

impl NetworkMeter for NetworkFinanceMeter {
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
