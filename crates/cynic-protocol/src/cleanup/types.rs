//! Cleanup agent types — pure shapes for organism self-maintenance.
//!
//! Pattern: types.rs (pures) → observe.rs (transducteur) → reconcile.rs (décision pure) → apply.rs (effet)
//!
//! Réutilisable par tout agent : merge, cleanup, debt, worktree, ...

use serde::{Deserialize, Serialize};

pub const CLEANUP_OBSERVATION_SCHEMA: &str = "cynic.cleanup_observation.v1";
pub const CLEANUP_DECISION_SCHEMA: &str = "cynic.cleanup_decision.v1";
pub const CLEANUP_PROOF_SCHEMA: &str = "cynic.cleanup_proof.v1";
pub const CLEANUP_ITEM_SCHEMA: &str = "cynic.cleanup_item.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupTargetKind {
    OrphanedCheckout,
    MergedBranch,
    StaleWorktree,
    IncompleteCheckout,
    BuildArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupItem {
    pub schema: String,
    pub target_kind: CleanupTargetKind,
    pub path: String,
    pub size_bytes: Option<u64>,
    pub last_modified_unix_s: Option<u64>,
    pub branch: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupObservation {
    pub schema: String,
    pub disk_used_percent: u8,
    pub disk_available_bytes: u64,
    pub disk_total_bytes: u64,
    pub items: Vec<CleanupItem>,
    pub reclaimable_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupDecision {
    pub schema: String,
    pub safe_to_delete: Vec<CleanupItem>,
    pub needs_review: Vec<CleanupItem>,
    pub keep: Vec<CleanupItem>,
    pub estimated_savings_bytes: u64,
    pub requires_human_approval: bool,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupProof {
    pub schema: String,
    pub deleted: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub space_freed_bytes: u64,
    pub disk_used_percent_after: u8,
}
