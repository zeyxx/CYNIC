//! Build artefact observation — 4-layer substrate for builder checkouts.
//!
//! Physical: raw disk device (via disk::PhysicalDisk, reused)
//! Volume:   mount <state> (disk::DiskFilesystem)
//! Usage:    checkouts + target/ consumption (CheckoutEntry)
//! Pressure: disk full / root-owned target (DiskPressure)
//!
//! No I/O here — pure types. Probe (parsing), adapter (I/O), and fleet rollup
//! live in cynic-host. The retention trio types live in cynic-protocol.

use serde::{Deserialize, Serialize};

pub use cynic_protocol::build::{CheckoutEntry, DiskPressure};

pub const BUILD_ARTEFACT_REPORT_SCHEMA: &str = cynic_protocol::retention::BUILD_ARTEFACT_REPORT_SCHEMA;
pub const BUILD_ARTEFACT_GC_PLAN_SCHEMA: &str = cynic_protocol::retention::BUILD_ARTEFACT_GC_PLAN_SCHEMA;
pub const BUILD_ARTEFACT_GC_RESULT_SCHEMA: &str = cynic_protocol::retention::BUILD_ARTEFACT_GC_RESULT_SCHEMA;

pub const FLEET_BUILD_ARTEFACT_SCHEMA: &str = cynic_protocol::retention::FLEET_BUILD_ARTEFACT_SCHEMA;

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
