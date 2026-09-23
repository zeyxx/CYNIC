//! Pure reconciliation engine for source worktree convergence.
//!
//! Transforms read-only inventory observations into deterministic, bounded convergence plans.
//! Strictly zero I/O and zero LLM dependency; delegates to `crate::source_worktree`.

use std::path::{Path, PathBuf};

use crate::source_worktree::{
    ObservedWorktreeSnapshot, WorktreeConvergencePlan, reconcile_worktree_convergence_pure,
};
use crate::{SourceWorktreeInventoryEntryState, SourceWorktreeInventoryObservation};

/// Reconciles observed worktrees against desired main head.
pub fn reconcile_source_worktrees(
    repository_root: &Path,
    inventory: &SourceWorktreeInventoryObservation,
    main_head_sha: &str,
    _active_claims_count: usize,
) -> WorktreeConvergencePlan {
    let mut snapshots = Vec::new();

    for entry in &inventory.entries {
        let is_active_root = entry.path_basis
            == crate::SourceWorktreeInventoryPathBasis::MainRepository
            || entry.relative_path.as_deref() == Some(".")
            || entry.relative_path.as_deref() == Some("")
            || entry.candidate_id == "cynic";

        let branch = entry
            .branch
            .as_deref()
            .map(|b| b.strip_prefix("refs/heads/").unwrap_or(b).to_string());

        let path = if is_active_root {
            repository_root.to_path_buf()
        } else {
            entry
                .relative_path
                .as_deref()
                .map(|r| {
                    let ws_dir = std::env::var_os("HOME")
                        .map(|h| PathBuf::from(h).join(".workspace"))
                        .filter(|p| p.exists())
                        .unwrap_or_else(|| PathBuf::from("/tmp/cynic-workspaces"));
                    ws_dir.join(r)
                })
                .unwrap_or_else(|| PathBuf::from(&entry.candidate_id))
        };

        let is_malformed = entry.state == SourceWorktreeInventoryEntryState::Malformed;
        let prunable = entry.git_prunable == Some(true) || is_malformed;
        let reason = if is_malformed {
            Some("Malformed worktree record".to_string())
        } else {
            entry.prunable_reason.clone()
        };

        snapshots.push(ObservedWorktreeSnapshot {
            path,
            branch,
            head_sha: entry.head.clone().unwrap_or_default(),
            is_active_root,
            dirty_file_count: 0,
            git_prunable: prunable,
            prunable_reason: reason,
        });
    }

    // Default branch is resolved or defaults to main
    let default_branch = "main";
    let empty_leases: [String; 0] = [];

    reconcile_worktree_convergence_pure(
        repository_root.to_path_buf(),
        default_branch,
        main_head_sha,
        &snapshots,
        &empty_leases,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_worktree::WorktreeConvergenceAction;
    use crate::{SourceWorktreeInventoryEntry, SourceWorktreeInventoryPathBasis};

    #[test]
    fn reconciles_prunable_worktree_into_prune_action() {
        let repo_root = Path::new("/var/lib/cynic/repo");
        let inventory = SourceWorktreeInventoryObservation {
            schema: crate::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
            repository_id: "cynic".to_string(),
            node_id: "node-1".to_string(),
            observed_at_ms: 1700000000000,
            entries: vec![SourceWorktreeInventoryEntry {
                candidate_id: "wt-zombie-1".to_string(),
                evidence_digest: "sha256:abcd".to_string(),
                state: SourceWorktreeInventoryEntryState::Observed,
                unknown_attribute_count: 0,
                path_basis: SourceWorktreeInventoryPathBasis::WorkspaceLayout,
                relative_path: Some("agent-zombie-1".to_string()),
                head: Some("head-111".to_string()),
                branch: None,
                bare: Some(false),
                detached_head: Some(true),
                clean: Some(true),
                observed_principal: None,
                path_is_symlink: None,
                git_registered: Some(true),
                locked: Some(false),
                lock_reason: None,
                git_prunable: Some(true),
                prunable_reason: Some("git directory missing".to_string()),
                unknowns: vec![],
            }],
            evidence_refs: vec![],
            observation_unknowns: vec![],
        };

        let plan = reconcile_source_worktrees(repo_root, &inventory, "main-sha-000", 1);
        assert_eq!(plan.summary.safe_prune_ready_count, 1);
        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            WorktreeConvergenceAction::PruneZombieWorkspace { reason, .. } => {
                assert!(reason.contains("git directory missing"));
            }
            _ => panic!("Expected PruneZombieWorkspace action"),
        }
    }
}
