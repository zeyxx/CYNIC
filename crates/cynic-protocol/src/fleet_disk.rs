//! Fleet-wide disk observation report type.
//!
//! Produced by `FleetDiskObserver` (in `cynic-host`) which iterates declared
//! nodes and shells `df` out to each via SSH. The CLI and MCP surfaces project
//! from this canonical type so policy/aggregation stays in the crate, not the
//! surface.
//!
//! Note: per-node detail is a *full* `DiskReport` returned by that node's own
//! `cynic runtime disk` when reachable; when the SSH probe fails we record a
//! residual unknown so no silent degradation occurs.

use serde::{Deserialize, Serialize};

use crate::disk::DiskReport;
use crate::task_pipeline::EnergyBudget;

pub const FLEET_DISK_SCHEMA: &str = "cynic.fleet_disk.v1";

/// One node's disk observation, either observed or failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum FleetDiskNode {
    /// The node answered and returned a disk report.
    Observed {
        node_id: String,
        role: String,
        ip: String,
        report: DiskReport,
    },
    /// The node could not be reached / `df` failed.
    Failed {
        node_id: String,
        role: String,
        ip: String,
        error: String,
    },
}

/// Aggregated fleet disk report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetDiskReport {
    pub schema: String,
    pub corpus_unix_s: u64,
    pub total_nodes: usize,
    pub observed_nodes: usize,
    pub failed_nodes: usize,
    pub nodes: Vec<FleetDiskNode>,
    /// Residual risks / unknowns not captured per-node (e.g. parse failures).
    pub residual_unknowns: Vec<String>,
    #[serde(default)]
    pub budget: Option<EnergyBudget>,
    #[serde(default)]
    pub poll_cost_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_stable() {
        assert_eq!(FLEET_DISK_SCHEMA, "cynic.fleet_disk.v1");
    }
}
