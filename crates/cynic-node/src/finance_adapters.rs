//! Finance adapters for cynic-node.
//!
//! Implements cynic-finance TimeMeter port using process uptime and workload timing.

use cynic_finance::ports::{MeterError, TimeMeter};
use std::time::Instant;

/// Time meter — measures wall-clock seconds since a reference point.
///
/// In production, this would be backed by a workload's start timestamp
/// from `cynic-workload`. For now, it uses an in-process `Instant`.
pub struct NodeTimeMeter {
    start: Instant,
}

impl NodeTimeMeter {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for NodeTimeMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeMeter for NodeTimeMeter {
    fn measure_seconds(&self, _agent_id: &str) -> Result<u64, MeterError> {
        Ok(self.start.elapsed().as_secs())
    }
}
