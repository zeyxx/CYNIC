//! Pure reconciliation engine for source worktree convergence.
//!
//! Transforms read-only inventory observations into deterministic, bounded convergence plans.
//! Strictly zero I/O and zero LLM dependency.

use std::path::{Path, PathBuf};

use cynic_protocol::{SourceWorktreeInventoryEntryState, SourceWorktreeInventoryObservation};

use super::types::{
    QuarantineReason, SOURCE_WORKTREE_RECONCILE_SCHEMA, WorktreeConvergenceAction,
    WorktreeConvergencePlan, WorktreeConvergenceSummary,
};

/// Pure function reconciling observed worktrees against desired main head.
pub fn reconcile_source_worktrees(
    repository_root: &Path,
    inventory: &SourceWorktreeInventoryObservation,
    main_head_sha: &str,
    active_claims_count: usize,
) -> WorktreeConvergencePlan {
    let mut actions = Vec::new();
    let mut summary = WorktreeConvergenceSummary::default();

    for entry in &inventory.entries {
        if entry.state == SourceWorktreeInventoryEntryState::Malformed {
            actions.push(WorktreeConvergenceAction::EscalateToHuman {
                branch: entry.branch.clone().unwrap_or_default(),
                conflicting_paths: vec![entry.candidate_id.clone()],
                intent_summary: "Malformed worktree porcelain record detected".to_string(),
            });
            summary.human_escalations_count += 1;
            continue;
        }

        if entry.git_prunable == Some(true) {
            let workspace_path = entry
                .relative_path
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(&entry.candidate_id));
            actions.push(WorktreeConvergenceAction::PruneZombieWorkspace {
                workspace_path,
                branch: entry.branch.clone(),
                lease_id: None,
                reason: entry
                    .prunable_reason
                    .clone()
                    .unwrap_or_else(|| "Worktree marked prunable by git inventory".to_string()),
            });
            summary.pruned_zombies_count += 1;
            continue;
        }

        if entry.locked == Some(true) {
            if let Some(reason) = &entry.lock_reason {
                if reason.contains("quarantine") || reason.contains("semantic_break") {
                    actions.push(WorktreeConvergenceAction::QuarantineBranch {
                        branch: entry.branch.clone().unwrap_or_default(),
                        reason: QuarantineReason::CausalBreach {
                            details: reason.clone(),
                        },
                        quarantine_ref: format!("refs/quarantine/{}", entry.candidate_id),
                    });
                    summary.quarantined_count += 1;
                    continue;
                }
            }
        }

        if let Some(branch) = &entry.branch {
            // Skip main branch itself
            if branch == "main" || branch == "master" || branch.ends_with("/main") {
                continue;
            }

            let workspace_path = entry
                .relative_path
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(&entry.candidate_id));

            let branch_head = entry.head.as_deref().unwrap_or("");

            if branch_head == main_head_sha {
                // Already at main HEAD -> FastForwardMerge
                actions.push(WorktreeConvergenceAction::FastForwardMerge {
                    branch: branch.clone(),
                    target_commit_sha: branch_head.to_string(),
                    lease_id: None,
                });
                summary.fast_forward_ready_count += 1;
            } else if !branch_head.is_empty() {
                // Diverged -> Rebase in Scratch Validator
                actions.push(WorktreeConvergenceAction::RebaseInScratchValidator {
                    branch: branch.clone(),
                    original_sha: branch_head.to_string(),
                    onto_main_sha: main_head_sha.to_string(),
                    workspace_path,
                });
                summary.rebase_required_count += 1;
            }
        }
    }

    WorktreeConvergencePlan {
        schema: SOURCE_WORKTREE_RECONCILE_SCHEMA,
        repository_root: repository_root.to_path_buf(),
        main_head_sha: main_head_sha.to_string(),
        observed_worktrees_count: inventory.entries.len(),
        active_claims_count,
        actions,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        SourceWorktreeInventoryEntry, SourceWorktreeInventoryEntryState,
        SourceWorktreeInventoryPathBasis,
    };

    #[test]
    fn reconciles_prunable_worktree_into_prune_action() {
        let repo_root = Path::new("/var/lib/cynic/repo");
        let inventory = SourceWorktreeInventoryObservation {
            schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
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
                branch: Some("agent/hermes/dead-branch".to_string()),
                observed_principal: Some("cynic".to_string()),
                path_is_symlink: Some(false),
                git_registered: Some(true),
                git_prunable: Some(true),
                detached_head: Some(false),
                clean: Some(true),
                locked: Some(false),
                bare: Some(false),
                lock_reason: None,
                prunable_reason: Some("git directory missing".to_string()),
                unknowns: vec![],
            }],
            evidence_refs: vec![],
            observation_unknowns: vec![],
        };

        let plan = reconcile_source_worktrees(repo_root, &inventory, "main-sha-000", 1);
        assert_eq!(plan.summary.pruned_zombies_count, 1);
        assert_eq!(plan.actions.len(), 1);
        match &plan.actions[0] {
            WorktreeConvergenceAction::PruneZombieWorkspace { reason, .. } => {
                assert!(reason.contains("git directory missing"));
            }
            _ => panic!("Expected PruneZombieWorkspace action"),
        }
    }
}
