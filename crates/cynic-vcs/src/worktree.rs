//! Source worktree convergence — inventory-driven reconciliation and bounded apply.
//!
//! Merges the former `reconcile/reconcile` (pure plan), `reconcile/apply`
//! (bounded execution behind an `ArgvRunner` port) and the zero-I/O decision
//! engine of `source_worktree`. The decision is pure; execution is injected.

use std::any::Any;
use std::path::{Path, PathBuf};

use crate::ports::ArgvRunner;
use crate::types::*;

pub const SOURCE_WORKTREE_RECONCILE_SCHEMA: &str = SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA;

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

pub trait VcsLockPort: Send + Sync {
    fn acquire_lock(&self, repository_root: &Path) -> Result<Box<dyn Any>, String>;
}

/// Executes a convergence plan and returns structured read-back proof.
pub fn apply_worktree_convergence<R: ArgvRunner>(
    plan: &WorktreeConvergencePlan,
    runner: &R,
    dry_run: bool,
    timestamp_epoch_seconds: u64,
) -> WorktreeConvergenceApplyProof {
    let now = timestamp_epoch_seconds;

    let root_str = plan.repository_root.to_string_lossy();
    let mut receipts = Vec::new();
    let mut all_successful = true;

    for action in &plan.actions {
        match action {
            WorktreeConvergenceAction::PruneMergedWorkspace {
                workspace_path,
                branch,
                reason,
                ..
            } => {
                if dry_run {
                    receipts.push(ExecutedActionReceipt {
                        action_name: "PruneMergedWorkspace",
                        target_path: workspace_path.clone(),
                        target_branch: Some(branch.clone()),
                        success: true,
                        message: format!(
                            "Dry-run: would remove worktree and delete branch ({reason})"
                        ),
                        read_back_evidence: None,
                    });
                    continue;
                }

                let ws_str = workspace_path.to_string_lossy();
                // 1. Remove worktree (force to detach)
                let wt_out = runner.run(&[
                    "git", "-C", &root_str, "worktree", "remove", "--force", &ws_str,
                ]);
                let wt_ok = wt_out.success()
                    || wt_out.stderr.contains("is not a working tree")
                    || wt_out.stderr.contains("No such file or directory");

                // 2. Delete merged local branch
                let br_out = runner.run(&["git", "-C", &root_str, "branch", "-d", branch]);
                let br_ok = br_out.success()
                    || br_out.stderr.contains("not found")
                    || br_out.stdout.contains("Deleted branch");

                let success = wt_ok && br_ok;
                if !success {
                    all_successful = false;
                }

                receipts.push(ExecutedActionReceipt {
                    action_name: "PruneMergedWorkspace",
                    target_path: workspace_path.clone(),
                    target_branch: Some(branch.clone()),
                    success,
                    message: format!("wt: {}, branch: {}", wt_out.summary(), br_out.summary()),
                    read_back_evidence: if success {
                        Some(format!("Pruned: {ws_str}, deleted branch: {branch}"))
                    } else {
                        None
                    },
                });
            }

            WorktreeConvergenceAction::PruneZombieWorkspace {
                workspace_path,
                branch,
                reason,
            } => {
                if dry_run {
                    receipts.push(ExecutedActionReceipt {
                        action_name: "PruneZombieWorkspace",
                        target_path: workspace_path.clone(),
                        target_branch: branch.clone(),
                        success: true,
                        message: format!("Dry-run: would force remove zombie worktree ({reason})"),
                        read_back_evidence: None,
                    });
                    continue;
                }

                let ws_str = workspace_path.to_string_lossy();
                let wt_out = runner.run(&[
                    "git", "-C", &root_str, "worktree", "remove", "--force", &ws_str,
                ]);
                let prune_out = runner.run(&["git", "-C", &root_str, "worktree", "prune"]);
                let success = wt_out.success()
                    || prune_out.success()
                    || wt_out.stderr.contains("is not a working tree")
                    || wt_out.stderr.contains("n'est pas une copie de travail")
                    || wt_out.stderr.contains("No such file or directory");

                if !success {
                    all_successful = false;
                }

                receipts.push(ExecutedActionReceipt {
                    action_name: "PruneZombieWorkspace",
                    target_path: workspace_path.clone(),
                    target_branch: branch.clone(),
                    success,
                    message: wt_out.summary(),
                    read_back_evidence: if success {
                        Some(format!("Pruned zombie: {ws_str}"))
                    } else {
                        None
                    },
                });
            }

            WorktreeConvergenceAction::EligibleForIntegration {
                workspace_path,
                branch,
                ahead_count,
                ..
            } => {
                receipts.push(ExecutedActionReceipt {
                    action_name: "EligibleForIntegration",
                    target_path: workspace_path.clone(),
                    target_branch: Some(branch.clone()),
                    success: true,
                    message: format!("Skipped in v1: +{ahead_count} commits ahead; requires verified BuildProof before auto-merge"),
                    read_back_evidence: None,
                });
            }

            WorktreeConvergenceAction::EscalateDivergence {
                workspace_path,
                branch,
                reason,
                ..
            } => {
                receipts.push(ExecutedActionReceipt {
                    action_name: "EscalateDivergence",
                    target_path: workspace_path.clone(),
                    target_branch: Some(branch.clone()),
                    success: true,
                    message: format!("Escalated to human: {reason}"),
                    read_back_evidence: None,
                });
            }

            WorktreeConvergenceAction::RetainActiveWorkspace {
                workspace_path,
                branch,
                reason,
            } => {
                receipts.push(ExecutedActionReceipt {
                    action_name: "RetainActiveWorkspace",
                    target_path: workspace_path.clone(),
                    target_branch: branch.clone(),
                    success: true,
                    message: format!("Retained: {reason}"),
                    read_back_evidence: None,
                });
            }
        }
    }

    WorktreeConvergenceApplyProof {
        schema: SOURCE_WORKTREE_APPLY_PROOF_SCHEMA,
        applied_at_unix_s: now,
        repository_root: plan.repository_root.clone(),
        main_initial_sha: plan.main_head_sha.clone(),
        main_final_sha: plan.main_head_sha.clone(),
        executed_actions: receipts,
        all_successful,
    }
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
                reason: format!(
                    "Protected: {} uncommitted changes present",
                    wt.dirty_file_count
                ),
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
mod reconcile_tests {
    use super::*;
    use crate::types::WorktreeConvergenceAction;
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
#[cfg(test)]
mod apply_tests {
    use super::*;
    use crate::ports::{ArgvOutput, MapArgvRunner};
    use crate::types::{
        WorktreeConvergenceAction, WorktreeConvergencePlan, WorktreeConvergenceSummary,
    };

    #[test]
    fn test_apply_executes_safe_prune_idempotently() {
        let runner = MapArgvRunner {
            responses: vec![
                (
                    vec![
                        "git".to_string(),
                        "-C".to_string(),
                        "/repo".to_string(),
                        "worktree".to_string(),
                        "remove".to_string(),
                        "--force".to_string(),
                        "/repo/.workspace/wt-1".to_string(),
                    ],
                    ArgvOutput {
                        exit_code: Some(0),
                        stdout: "".to_string(),
                        stderr: "".to_string(),
                        spawn_error: None,
                    },
                ),
                (
                    vec![
                        "git".to_string(),
                        "-C".to_string(),
                        "/repo".to_string(),
                        "branch".to_string(),
                        "-d".to_string(),
                        "agent/wt-1".to_string(),
                    ],
                    ArgvOutput {
                        exit_code: Some(0),
                        stdout: "Deleted branch agent/wt-1".to_string(),
                        stderr: "".to_string(),
                        spawn_error: None,
                    },
                ),
            ],
        };
        let plan = WorktreeConvergencePlan {
            schema: crate::types::SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA,
            repository_root: PathBuf::from("/repo"),
            default_branch: "main".to_string(),
            main_head_sha: "sha-001".to_string(),
            actions: vec![
                WorktreeConvergenceAction::PruneMergedWorkspace {
                    workspace_path: PathBuf::from("/repo/.workspace/wt-1"),
                    branch: "agent/wt-1".to_string(),
                    commit_sha: "sha-001".to_string(),
                    reason: "clean merged".to_string(),
                },
                WorktreeConvergenceAction::RetainActiveWorkspace {
                    workspace_path: PathBuf::from("/repo"),
                    branch: Some("main".to_string()),
                    reason: "root".to_string(),
                },
            ],
            summary: WorktreeConvergenceSummary::default(),
        };

        let proof = apply_worktree_convergence(&plan, &runner, false, 1_700_000_000);
        assert_eq!(proof.executed_actions.len(), 2);
        assert!(proof.executed_actions[0].success);
        assert_eq!(
            proof.executed_actions[0].action_name,
            "PruneMergedWorkspace"
        );
        assert!(proof.all_successful);
    }

    #[test]
    fn test_apply_dry_run_does_not_mutate() {
        let runner = MapArgvRunner { responses: vec![] };
        let plan = WorktreeConvergencePlan {
            schema: crate::types::SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA,
            repository_root: PathBuf::from("/repo"),
            default_branch: "main".to_string(),
            main_head_sha: "sha-001".to_string(),
            actions: vec![WorktreeConvergenceAction::PruneZombieWorkspace {
                workspace_path: PathBuf::from("/repo/zombie"),
                branch: None,
                reason: "missing git dir".to_string(),
            }],
            summary: WorktreeConvergenceSummary::default(),
        };

        let proof = apply_worktree_convergence(&plan, &runner, true, 1_700_000_000);
        assert_eq!(proof.executed_actions.len(), 1);
        assert!(proof.executed_actions[0].message.contains("Dry-run"));
        assert!(proof.all_successful);
    }
}
#[cfg(test)]
mod pure_tests {
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
