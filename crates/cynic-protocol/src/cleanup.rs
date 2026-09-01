//! Cleanup agent — pure types for organism self-maintenance.
//!
//! The cleanup agent observes disk state, identifies orphaned resources,
//! proposes deletions, and proves cleanup results.
//!
//! Architecture follows the Organic Opportunity pattern:
//!   Protocol (pure types) → Observe (argv) → Reconcile (pure) → Apply → Proof

use serde::Deserialize;
use serde::Serialize;

pub const CLEANUP_OBSERVATION_SCHEMA: &str = "cynic.cleanup_observation.v1";
pub const CLEANUP_DECISION_SCHEMA: &str = "cynic.cleanup_decision.v1";
pub const CLEANUP_PROOF_SCHEMA: &str = "cynic.cleanup_proof.v1";
pub const CLEANUP_ITEM_SCHEMA: &str = "cynic.cleanup_item.v1";

/// What kind of resource can be cleaned up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupTargetKind {
    /// An orphaned builder checkout (no matching remote branch).
    OrphanedCheckout,
    /// A merged branch that can be safely deleted.
    MergedBranch,
    /// A stale worktree (no recent activity).
    StaleWorktree,
    /// A failed/incomplete checkout (< 1MB).
    IncompleteCheckout,
    /// Target build artifacts older than N days.
    BuildArtifact,
}

/// A single cleanup candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupItem {
    pub schema: String,
    /// What kind of resource this is.
    pub target_kind: CleanupTargetKind,
    /// Path or identifier for the resource.
    pub path: String,
    /// Size in bytes (if known).
    pub size_bytes: Option<u64>,
    /// Last modification time (unix seconds, if known).
    pub last_modified_unix_s: Option<u64>,
    /// Associated branch name (if applicable).
    pub branch: Option<String>,
    /// Why this item was flagged.
    pub rationale: String,
}

/// Observation of disk and resource state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupObservation {
    pub schema: String,
    /// Disk usage percentage (0-100).
    pub disk_used_percent: u8,
    /// Disk available in bytes.
    pub disk_available_bytes: u64,
    /// Total disk size in bytes.
    pub disk_total_bytes: u64,
    /// Items identified for cleanup.
    pub items: Vec<CleanupItem>,
    /// Total reclaimable bytes (sum of item sizes).
    pub reclaimable_bytes: u64,
}

/// Pure decision from observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupDecision {
    pub schema: String,
    /// Items to delete.
    pub safe_to_delete: Vec<CleanupItem>,
    /// Items requiring human review.
    pub needs_review: Vec<CleanupItem>,
    /// Items to keep.
    pub keep: Vec<CleanupItem>,
    /// Estimated space savings in bytes.
    pub estimated_savings_bytes: u64,
    /// Whether human approval is required.
    pub requires_human_approval: bool,
    /// Rationale for the decision.
    pub rationale: String,
}

/// Pure reconciliation: decide what to clean from observation.
///
/// Rules:
/// - OrphanedCheckout (no matching remote branch) → SafeDelete
/// - MergedBranch (0 commits ahead of main) → SafeDelete
/// - IncompleteCheckout (< 1MB) → SafeDelete
/// - StaleWorktree (> 30 days inactive) → SafeDelete
/// - All others → NeedsReview
pub fn reconcile_cleanup(obs: &CleanupObservation) -> CleanupDecision {
    let mut safe_to_delete = Vec::new();
    let mut needs_review = Vec::new();
    let mut keep = Vec::new();

    for item in &obs.items {
        match item.target_kind {
            CleanupTargetKind::OrphanedCheckout
            | CleanupTargetKind::MergedBranch
            | CleanupTargetKind::IncompleteCheckout => {
                safe_to_delete.push(item.clone());
            }
            CleanupTargetKind::StaleWorktree => {
                // Only safe if truly stale (> 30 days)
                let is_stale = item
                    .last_modified_unix_s
                    .map(|t| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        now.saturating_sub(t) > 30 * 24 * 60 * 60
                    })
                    .unwrap_or(false);
                if is_stale {
                    safe_to_delete.push(item.clone());
                } else {
                    needs_review.push(item.clone());
                }
            }
            CleanupTargetKind::BuildArtifact => {
                needs_review.push(item.clone());
            }
        }
    }

    let safe_count = safe_to_delete.len();
    let review_count = needs_review.len();
    let estimated_savings_bytes: u64 = safe_to_delete.iter().filter_map(|i| i.size_bytes).sum();

    CleanupDecision {
        schema: CLEANUP_DECISION_SCHEMA.to_string(),
        safe_to_delete,
        needs_review,
        keep,
        estimated_savings_bytes,
        requires_human_approval: false,
        rationale: format!(
            "Identified {} items safe to delete, {} need review",
            safe_count, review_count
        ),
    }
}

/// Proof of cleanup execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupProof {
    pub schema: String,
    /// Items that were successfully deleted.
    pub deleted: Vec<String>,
    /// Items that failed to delete.
    pub failed: Vec<(String, String)>,
    /// Space freed in bytes.
    pub space_freed_bytes: u64,
    /// Disk usage after cleanup.
    pub disk_used_percent_after: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconcile_identifies_orphans_as_safe_to_delete() {
        let obs = CleanupObservation {
            schema: CLEANUP_OBSERVATION_SCHEMA.to_string(),
            disk_used_percent: 95,
            disk_available_bytes: 2_000_000_000,
            disk_total_bytes: 40_000_000_000,
            items: vec![
                CleanupItem {
                    schema: CLEANUP_ITEM_SCHEMA.to_string(),
                    target_kind: CleanupTargetKind::OrphanedCheckout,
                    path: "/var/lib/cynic/checkouts/abc123".to_string(),
                    size_bytes: Some(2_500_000_000),
                    last_modified_unix_s: Some(1724000000),
                    branch: None,
                    rationale: "No matching remote branch".to_string(),
                },
                CleanupItem {
                    schema: CLEANUP_ITEM_SCHEMA.to_string(),
                    target_kind: CleanupTargetKind::MergedBranch,
                    path: "origin/agent/old-feature".to_string(),
                    size_bytes: None,
                    last_modified_unix_s: None,
                    branch: Some("origin/agent/old-feature".to_string()),
                    rationale: "Merged into main".to_string(),
                },
            ],
            reclaimable_bytes: 2_500_000_000,
        };

        let decision = reconcile_cleanup(&obs);
        assert_eq!(decision.safe_to_delete.len(), 2);
        assert!(decision.needs_review.is_empty());
        assert_eq!(decision.estimated_savings_bytes, 2_500_000_000);
    }

    #[test]
    fn reconcile_flags_build_artifacts_for_review() {
        let obs = CleanupObservation {
            schema: CLEANUP_OBSERVATION_SCHEMA.to_string(),
            disk_used_percent: 80,
            disk_available_bytes: 8_000_000_000,
            disk_total_bytes: 40_000_000_000,
            items: vec![CleanupItem {
                schema: CLEANUP_ITEM_SCHEMA.to_string(),
                target_kind: CleanupTargetKind::BuildArtifact,
                path: "target/debug/old-binary".to_string(),
                size_bytes: Some(500_000_000),
                last_modified_unix_s: Some(1724000000),
                branch: None,
                rationale: "Build artifact older than 30 days".to_string(),
            }],
            reclaimable_bytes: 500_000_000,
        };

        let decision = reconcile_cleanup(&obs);
        assert!(decision.safe_to_delete.is_empty());
        assert_eq!(decision.needs_review.len(), 1);
    }
}
