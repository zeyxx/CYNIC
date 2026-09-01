//! Cleanup agent — pure reconciliation (decision function).
//!
//! No I/O here. Consumes `CleanupObservation`, produces `CleanupDecision`.
//!
//! Rules:
//! - OrphanedCheckout → SafeDelete
//! - MergedBranch → SafeDelete
//! - IncompleteCheckout (< 1MB) → SafeDelete
//! - StaleWorktree (> 30 days inactive) → SafeDelete
//! - Otherwise → NeedsReview

use crate::cleanup::types::*;

/// Pure decision function from observation to strategy.
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

    #[test]
    fn reconcile_keeps_recent_worktree() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let obs = CleanupObservation {
            schema: CLEANUP_OBSERVATION_SCHEMA.to_string(),
            disk_used_percent: 80,
            disk_available_bytes: 8_000_000_000,
            disk_total_bytes: 40_000_000_000,
            items: vec![CleanupItem {
                schema: CLEANUP_ITEM_SCHEMA.to_string(),
                target_kind: CleanupTargetKind::StaleWorktree,
                path: "/var/lib/cynic/checkouts/active-branch".to_string(),
                size_bytes: Some(1_000_000_000),
                last_modified_unix_s: Some(now - 60), // 1 minute ago
                branch: Some("origin/agent/active".to_string()),
                rationale: "Recently active".to_string(),
            }],
            reclaimable_bytes: 0,
        };

        let decision = reconcile_cleanup(&obs);
        assert!(decision.safe_to_delete.is_empty());
        assert_eq!(decision.needs_review.len(), 1);
    }

    #[test]
    fn reconcile_deletes_truly_stale_worktree() {
        let obs = CleanupObservation {
            schema: CLEANUP_OBSERVATION_SCHEMA.to_string(),
            disk_used_percent: 95,
            disk_available_bytes: 2_000_000_000,
            disk_total_bytes: 40_000_000_000,
            items: vec![CleanupItem {
                schema: CLEANUP_ITEM_SCHEMA.to_string(),
                target_kind: CleanupTargetKind::StaleWorktree,
                path: "/var/lib/cynic/checkouts/old-stale".to_string(),
                size_bytes: Some(3_000_000_000),
                last_modified_unix_s: Some(1724000000), // ~Sep 2024
                branch: None,
                rationale: "No activity in 30+ days".to_string(),
            }],
            reclaimable_bytes: 3_000_000_000,
        };

        let decision = reconcile_cleanup(&obs);
        assert_eq!(decision.safe_to_delete.len(), 1);
        assert_eq!(decision.estimated_savings_bytes, 3_000_000_000);
    }
}
