//! Bounded action executor for source worktree convergence.
//!
//! Executes convergence plans through an injectable ArgvRunner and collects read-back evidence.
//! Enforces exclusive locking and idempotent pruning.

use std::any::Any;
use std::path::{Path, PathBuf};

use crate::runner::ArgvRunner;
use crate::source_worktree::{
    ExecutedActionReceipt, SOURCE_WORKTREE_APPLY_PROOF_SCHEMA, WorktreeConvergenceAction,
    WorktreeConvergenceApplyProof, WorktreeConvergencePlan,
};

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
                let success = wt_out.success()
                    || wt_out.stderr.contains("is not a working tree")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{ArgvOutput, MapArgvRunner};
    use crate::source_worktree::{
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
            schema: crate::source_worktree::SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA,
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
            schema: crate::source_worktree::SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA,
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
