//! Pure worktree convergence data models and zero-I/O decision engine.
//!
//! Conforms to `contracts/foundation.json` §481 and `infra/SYSTEM_AGENTIC_MAPPING.md`.
//! ZERO I/O: operates strictly on snapshots, producing deterministic plans.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA: &str =
    "cynic.source_worktree.convergence_plan.v1";
pub const SOURCE_WORKTREE_APPLY_PROOF_SCHEMA: &str =
    "cynic.source_worktree.apply_proof.v1";

/// Snapshot of an observed worktree passed into the pure decision function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedWorktreeSnapshot {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub head_sha: String,
    pub is_active_root: bool,
    pub dirty_file_count: usize,
    pub git_prunable: bool,
    pub prunable_reason: Option<String>,
}

/// Discrete, bounded actions decidable by the pure reconciliation engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum WorktreeConvergenceAction {
    /// 1. Branch is clean, fully merged into main HEAD -> safe to remove worktree and delete local branch
    PruneMergedWorkspace {
        workspace_path: PathBuf,
        branch: String,
        commit_sha: String,
        reason: String,
    },
    /// 2. Branch is ahead of main with no conflicts -> eligible for integration
    EligibleForIntegration {
        workspace_path: PathBuf,
        branch: String,
        target_commit_sha: String,
        ahead_count: usize,
    },
    /// 3. Branch diverged from main -> requires explicit human review
    EscalateDivergence {
        workspace_path: PathBuf,
        branch: String,
        branch_head: String,
        onto_main_sha: String,
        reason: String,
    },
    /// 4. Worktree is marked prunable or detached without valid git dir -> safe to force remove
    PruneZombieWorkspace {
        workspace_path: PathBuf,
        branch: Option<String>,
        reason: String,
    },
    /// 5. Active session, uncommitted changes, or active coordination lease -> protected
    RetainActiveWorkspace {
        workspace_path: PathBuf,
        branch: Option<String>,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorktreeConvergenceSummary {
    pub total_observed: usize,
    pub safe_prune_ready_count: usize,
    pub integration_eligible_count: usize,
    pub retained_active_count: usize,
    pub escalation_required_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeConvergencePlan {
    pub schema: &'static str,
    pub repository_root: PathBuf,
    pub default_branch: String,
    pub main_head_sha: String,
    pub actions: Vec<WorktreeConvergenceAction>,
    pub summary: WorktreeConvergenceSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutedActionReceipt {
    pub action_name: &'static str,
    pub target_path: PathBuf,
    pub target_branch: Option<String>,
    pub success: bool,
    pub message: String,
    pub read_back_evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeConvergenceApplyProof {
    pub schema: &'static str,
    pub applied_at_unix_s: u64,
    pub repository_root: PathBuf,
    pub main_initial_sha: String,
    pub main_final_sha: String,
    pub executed_actions: Vec<ExecutedActionReceipt>,
    pub all_successful: bool,
}

/// Pure zero-I/O reconciliation function.
pub fn reconcile_worktree_convergence_pure(
    repository_root: PathBuf,
    default_branch: &str,
    main_head_sha: &str,
    worktrees: &[ObservedWorktreeSnapshot],
    active_lease_branches: &[String],
) -> WorktreeConvergencePlan {
    let mut actions = Vec::new();
    let mut summary = WorktreeConvergenceSummary {
        total_observed: worktrees.len(),
        ..Default::default()
    };

    for wt in worktrees {
        // 1. Current active root is always retained
        if wt.is_active_root {
            actions.push(WorktreeConvergenceAction::RetainActiveWorkspace {
                workspace_path: wt.path.clone(),
                branch: wt.branch.clone(),
                reason: "Current active working directory".to_string(),
            });
            summary.retained_active_count += 1;
            continue;
        }

        // 2. Git prunable zombies
        if wt.git_prunable {
            actions.push(WorktreeConvergenceAction::PruneZombieWorkspace {
                workspace_path: wt.path.clone(),
                branch: wt.branch.clone(),
                reason: wt
                    .prunable_reason
                    .clone()
                    .unwrap_or_else(|| "Git inventory marked worktree prunable".to_string()),
            });
            summary.safe_prune_ready_count += 1;
            continue;
        }

        // 3. Uncommitted modifications must never be destroyed
        if wt.dirty_file_count > 0 {
            actions.push(WorktreeConvergenceAction::RetainActiveWorkspace {
                workspace_path: wt.path.clone(),
                branch: wt.branch.clone(),
                reason: format!("Protected: {} uncommitted changes present", wt.dirty_file_count),
            });
            summary.retained_active_count += 1;
            continue;
        }

        if let Some(branch) = &wt.branch {
            // 4. Branch holds an active coordination lease
            if active_lease_branches.contains(branch) {
                actions.push(WorktreeConvergenceAction::RetainActiveWorkspace {
                    workspace_path: wt.path.clone(),
                    branch: Some(branch.clone()),
                    reason: "Protected: active coordination lease held".to_string(),
                });
                summary.retained_active_count += 1;
                continue;
            }

            // 5. Default branch itself (e.g. main / master)
            if branch == default_branch || branch == "main" || branch == "master" {
                actions.push(WorktreeConvergenceAction::RetainActiveWorkspace {
                    workspace_path: wt.path.clone(),
                    branch: Some(branch.clone()),
                    reason: "Protected: default branch checkout".to_string(),
                });
                summary.retained_active_count += 1;
                continue;
            }

            // 6. Branch is clean and at main HEAD -> safe to prune
            if wt.head_sha == main_head_sha {
                actions.push(WorktreeConvergenceAction::PruneMergedWorkspace {
                    workspace_path: wt.path.clone(),
                    branch: branch.clone(),
                    commit_sha: wt.head_sha.clone(),
                    reason: "Branch is fully merged into main HEAD and clean".to_string(),
                });
                summary.safe_prune_ready_count += 1;
            } else {
                // Diverged or ahead without proof
                actions.push(WorktreeConvergenceAction::EscalateDivergence {
                    workspace_path: wt.path.clone(),
                    branch: branch.clone(),
                    branch_head: wt.head_sha.clone(),
                    onto_main_sha: main_head_sha.to_string(),
                    reason: "Branch diverged from main HEAD; requires explicit review".to_string(),
                });
                summary.escalation_required_count += 1;
            }
        } else {
            // Detached worktree with no branch
            actions.push(WorktreeConvergenceAction::PruneZombieWorkspace {
                workspace_path: wt.path.clone(),
                branch: None,
                reason: "Detached worktree with no branch".to_string(),
            });
            summary.safe_prune_ready_count += 1;
        }
    }

    WorktreeConvergencePlan {
        schema: SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA,
        repository_root,
        default_branch: default_branch.to_string(),
        main_head_sha: main_head_sha.to_string(),
        actions,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_inventory() -> Vec<ObservedWorktreeSnapshot> {
        vec![
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/main"),
                branch: Some("main".to_string()),
                head_sha: "sha-main-001".to_string(),
                is_active_root: true,
                dirty_file_count: 0,
                git_prunable: false,
                prunable_reason: None,
            },
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/merged-wt"),
                branch: Some("agent/feature-merged".to_string()),
                head_sha: "sha-main-001".to_string(),
                is_active_root: false,
                dirty_file_count: 0,
                git_prunable: false,
                prunable_reason: None,
            },
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/dirty-wt"),
                branch: Some("agent/feature-dirty".to_string()),
                head_sha: "sha-dirty-002".to_string(),
                is_active_root: false,
                dirty_file_count: 3,
                git_prunable: false,
                prunable_reason: None,
            },
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/zombie-wt"),
                branch: None,
                head_sha: "sha-unknown".to_string(),
                is_active_root: false,
                dirty_file_count: 0,
                git_prunable: true,
                prunable_reason: Some("git directory missing".to_string()),
            },
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/leased-wt"),
                branch: Some("agent/leased-active".to_string()),
                head_sha: "sha-main-001".to_string(),
                is_active_root: false,
                dirty_file_count: 0,
                git_prunable: false,
                prunable_reason: None,
            },
        ]
    }

    #[test]
    fn test_reconcile_retains_active_root() {
        let items = make_test_inventory();
        let plan = reconcile_worktree_convergence_pure(
            PathBuf::from("/repo"),
            "main",
            "sha-main-001",
            &items,
            &[],
        );
        let active_action = plan.actions.iter().find(|a| {
            matches!(
                a,
                WorktreeConvergenceAction::RetainActiveWorkspace { reason, .. }
                    if reason.contains("Current active")
            )
        });
        assert!(active_action.is_some());
    }

    #[test]
    fn test_reconcile_prunes_clean_merged_worktree() {
        let items = make_test_inventory();
        let plan = reconcile_worktree_convergence_pure(
            PathBuf::from("/repo"),
            "main",
            "sha-main-001",
            &items,
            &[],
        );
        let merged_action = plan.actions.iter().find(|a| {
            matches!(
                a,
                WorktreeConvergenceAction::PruneMergedWorkspace { branch, .. }
                    if branch == "agent/feature-merged"
            )
        });
        assert!(merged_action.is_some());
        assert_eq!(plan.summary.safe_prune_ready_count, 3); // merged-wt + zombie-wt + unleased wt
    }

    #[test]
    fn test_reconcile_protects_dirty_worktree() {
        let items = make_test_inventory();
        let plan = reconcile_worktree_convergence_pure(
            PathBuf::from("/repo"),
            "main",
            "sha-main-001",
            &items,
            &[],
        );
        let dirty_action = plan.actions.iter().find(|a| {
            matches!(
                a,
                WorktreeConvergenceAction::RetainActiveWorkspace { reason, .. }
                    if reason.contains("uncommitted changes")
            )
        });
        assert!(dirty_action.is_some());
    }

    #[test]
    fn test_reconcile_protects_active_lease() {
        let items = make_test_inventory();
        let plan = reconcile_worktree_convergence_pure(
            PathBuf::from("/repo"),
            "main",
            "sha-main-001",
            &items,
            &["agent/leased-active".to_string()],
        );
        let leased_action = plan.actions.iter().find(|a| {
            matches!(
                a,
                WorktreeConvergenceAction::RetainActiveWorkspace { branch, reason, .. }
                    if branch.as_deref() == Some("agent/leased-active") && reason.contains("active coordination lease")
            )
        });
        assert!(leased_action.is_some());
    }

    #[test]
    fn test_reconcile_prunes_zombie() {
        let items = make_test_inventory();
        let plan = reconcile_worktree_convergence_pure(
            PathBuf::from("/repo"),
            "main",
            "sha-main-001",
            &items,
            &[],
        );
        let zombie_action = plan.actions.iter().find(|a| {
            matches!(
                a,
                WorktreeConvergenceAction::PruneZombieWorkspace { reason, .. }
                    if reason.contains("git directory missing")
            )
        });
        assert!(zombie_action.is_some());
    }
}
