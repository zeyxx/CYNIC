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
    /// True when removing this tree needs privilege.
    ///
    /// It is NOT enough for the tree ROOT to be root-owned: a root owned
    /// by this account can still contain entries owned by root or by a
    /// foreign uid, and `rm` fails on the first of them. Measured: two
    /// checkouts whose root was `cynic:cynic` held 9580 and 21
    /// foreign-owned entries, were planned as not needing privilege, and
    /// then failed to delete. The flag therefore covers the CONTENT.
    pub is_root_owned: bool,
    /// True when the tree contains entries this account does not own.
    ///
    /// The observation carries it so the plan can mark the item as
    /// privilege-requiring before the delete is attempted, instead of
    /// discovering it as a partial failure.
    pub contains_foreign_owned: bool,
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
