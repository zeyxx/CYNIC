//! Build artefact observation — 4-layer substrate for builder checkouts.
//!
//! Physical: raw disk device (via disk::PhysicalDisk, reused)
//! Volume:   mount /var/lib/cynic (disk::DiskFilesystem)
//! Usage:    checkouts + target/ consumption (CheckoutEntry)
//! Pressure: disk full / root-owned target (DiskPressure)
//!
//! No I/O here — pure types. Probe (parsing), adapter (I/O), and fleet rollup
//! live in cynic-host.

use serde::{Deserialize, Serialize};

use crate::disk::{DiskFilesystem, PhysicalDisk};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildArtefactReport {
    pub schema: String,
    pub timestamp: String,
    pub physical_disks: Vec<PhysicalDisk>,
    pub filesystems: Vec<DiskFilesystem>,
    pub checkouts: Vec<CheckoutEntry>,
    pub total_checkout_bytes: u64,
    pub total_checkout_human: String,
    pub target_bytes: u64,
    pub target_human: String,
    pub pressure: DiskPressure,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildArtefactGcItem {
    pub id: String,
    pub label: String,
    pub path: String,
    pub size_human: String,
    pub size_bytes: u64,
    pub reason: String,
    pub requires_privilege: bool,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildArtefactGcPlan {
    pub schema: String,
    pub items: Vec<BuildArtefactGcItem>,
    pub total_reclaimable_bytes: u64,
    pub total_reclaimable_human: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildArtefactGcStepExecution {
    pub item_id: String,
    pub label: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildArtefactGcResult {
    pub schema: String,
    pub timestamp: String,
    pub success: bool,
    pub items_executed: Vec<BuildArtefactGcStepExecution>,
    pub total_reclaimed_bytes: u64,
    pub total_reclaimed_human: String,
}
