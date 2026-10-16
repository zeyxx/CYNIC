//! Build artefact observation — 4-layer substrate for builder checkouts.
//!
//! Physical: raw disk device (via disk::PhysicalDisk, reused)
//! Volume:   mount /var/lib/cynic (disk::DiskFilesystem)
//! Usage:    checkouts + target/ consumption (CheckoutEntry)
//! Pressure: disk full / root-owned target (DiskPressure)
//!
//! No I/O here — pure types. Probe (parsing), adapter (I/O), and fleet rollup
//! live in cynic-host. The retention trio types live in cynic-protocol.

use serde::{Deserialize, Serialize};

pub const BUILD_ARTEFACT_REPORT_SCHEMA: &str = "cynic.build_artefact_report.v1";
pub const BUILD_ARTEFACT_GC_PLAN_SCHEMA: &str = "cynic.build_artefact_gc_plan.v1";
pub const BUILD_ARTEFACT_GC_RESULT_SCHEMA: &str = "cynic.build_artefact_gc_result.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutEntry {
    pub path: String,
    pub sha: String,
    pub branch: String,
    pub age_secs: Option<u64>,
    pub size_bytes: u64,
    pub size_human: String,
    pub owner_uid: u32,
    pub owner_gid: u32,
    pub is_orphan: bool,
    pub is_root_owned: bool,
    pub remote_exists: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskPressure {
    pub avail_bytes: u64,
    pub avail_human: String,
    pub use_pct: u8,
    pub is_pressured: bool,
    pub root_owned_target_count: usize,
}

pub const FLEET_BUILD_ARTEFACT_SCHEMA: &str = "cynic.fleet_build_artefact.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum FleetBuildArtefactNode {
    Observed {
        node_id: String,
        role: String,
        ip: String,
        report: cynic_protocol::retention::RetentionObservation,
    },
    Failed {
        node_id: String,
        role: String,
        ip: String,
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetBuildArtefactReport {
    pub schema: String,
    pub corpus_unix_s: u64,
    pub total_nodes: usize,
    pub observed_nodes: usize,
    pub failed_nodes: usize,
    pub nodes: Vec<FleetBuildArtefactNode>,
    pub residual_unknowns: Vec<String>,
    #[serde(default)]
    pub budget: Option<cynic_protocol::task_pipeline::EnergyBudget>,
    #[serde(default)]
    pub poll_cost_ms: Option<u64>,
}
