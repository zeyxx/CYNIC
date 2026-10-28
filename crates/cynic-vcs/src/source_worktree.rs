//! Pure worktree convergence data models and zero-I/O decision engine.
//!
//! Conforms to `contracts/foundation.json` §481 and `infra/workspace-paths.json`.
//! ZERO I/O: operates strictly on snapshots, producing deterministic plans.

use crate::runner::ArgvRunner;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA: &str =
    "cynic.source_worktree.convergence_plan.v1";
pub const SOURCE_WORKTREE_APPLY_PROOF_SCHEMA: &str = "cynic.source_worktree.apply_proof.v1";

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
    /// Directory mtime in epoch ms. `None` = unknown (no grace granted).
    pub created_at_ms: Option<u64>,
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

pub type WorktreeConvergenceProof = WorktreeConvergenceApplyProof;

/// Pure zero-I/O reconciliation function.
/// Fresh worktrees are never pruned: an agent that just created its
/// workspace must not lose it to a concurrent convergence pass before it
/// records its first dirty file or lease. `None` age = unknown = no grace.
pub const WORKTREE_MATURATION_GRACE_MS: u64 = 600_000;

pub fn reconcile_worktree_convergence_pure(
    repository_root: PathBuf,
    default_branch: &str,
    main_head_sha: &str,
    worktrees: &[ObservedWorktreeSnapshot],
    active_lease_branches: &[String],
    now_ms: u64,
) -> WorktreeConvergencePlan {
    let mut actions = Vec::new();
    let mut summary = WorktreeConvergenceSummary {
        total_observed: worktrees.len(),
        ..Default::default()
    };

    for wt in worktrees {
        // 0. Maturation grace: a worktree younger than the grace window is
        // retained unconditionally, even if it looks merged or zombified.
        // Unknown age grants no grace (fail-closed toward pruning).
        let is_young = matches!(wt.created_at_ms, Some(t) if now_ms.saturating_sub(t) < WORKTREE_MATURATION_GRACE_MS);
        if is_young {
            actions.push(WorktreeConvergenceAction::RetainActiveWorkspace {
                workspace_path: wt.path.clone(),
                branch: wt.branch.clone(),
                reason: "Maturation grace period: worktree younger than 10 min".to_string(),
            });
            summary.retained_active_count += 1;
            continue;
        }

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
                created_at_ms: None,
            },
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/merged-wt"),
                branch: Some("agent/feature-merged".to_string()),
                head_sha: "sha-main-001".to_string(),
                is_active_root: false,
                dirty_file_count: 0,
                git_prunable: false,
                prunable_reason: None,
                created_at_ms: None,
            },
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/dirty-wt"),
                branch: Some("agent/feature-dirty".to_string()),
                head_sha: "sha-dirty-002".to_string(),
                is_active_root: false,
                dirty_file_count: 3,
                git_prunable: false,
                prunable_reason: None,
                created_at_ms: None,
            },
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/zombie-wt"),
                branch: None,
                head_sha: "sha-unknown".to_string(),
                is_active_root: false,
                dirty_file_count: 0,
                git_prunable: true,
                prunable_reason: Some("git directory missing".to_string()),
                created_at_ms: None,
            },
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/leased-wt"),
                branch: Some("agent/leased-active".to_string()),
                head_sha: "sha-main-001".to_string(),
                is_active_root: false,
                dirty_file_count: 0,
                git_prunable: false,
                prunable_reason: None,
                created_at_ms: None,
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
            1_700_000_000_000,
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
            1_700_000_000_000,
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
            1_700_000_000_000,
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
            1_700_000_000_000,
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
            1_700_000_000_000,
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

    fn young_inventory(now_ms: u64) -> Vec<ObservedWorktreeSnapshot> {
        vec![
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/fresh-merged"),
                branch: Some("agent/fresh".to_string()),
                head_sha: "sha-main-001".to_string(),
                is_active_root: false,
                dirty_file_count: 0,
                git_prunable: false,
                prunable_reason: None,
                created_at_ms: Some(now_ms - 60_000),
            },
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/fresh-zombie"),
                branch: None,
                head_sha: "sha-unknown".to_string(),
                is_active_root: false,
                dirty_file_count: 0,
                git_prunable: true,
                prunable_reason: Some("git directory missing".to_string()),
                created_at_ms: Some(now_ms - 30_000),
            },
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/old-zombie"),
                branch: None,
                head_sha: "sha-unknown".to_string(),
                is_active_root: false,
                dirty_file_count: 0,
                git_prunable: true,
                prunable_reason: Some("git directory missing".to_string()),
                created_at_ms: Some(now_ms - WORKTREE_MATURATION_GRACE_MS - 1),
            },
            ObservedWorktreeSnapshot {
                path: PathBuf::from("/repo/ageless-zombie"),
                branch: None,
                head_sha: "sha-unknown".to_string(),
                is_active_root: false,
                dirty_file_count: 0,
                git_prunable: true,
                prunable_reason: Some("git directory missing".to_string()),
                created_at_ms: None,
            },
        ]
    }

    #[test]
    fn young_worktrees_are_retained_despite_prune_signals() {
        let now_ms = 1_700_000_000_000;
        let items = young_inventory(now_ms);
        let plan = reconcile_worktree_convergence_pure(
            PathBuf::from("/repo"),
            "main",
            "sha-main-001",
            &items,
            &[],
            now_ms,
        );
        for path in ["/repo/fresh-merged", "/repo/fresh-zombie"] {
            let retained = plan.actions.iter().any(|a| {
                matches!(
                    a,
                    WorktreeConvergenceAction::RetainActiveWorkspace {
                        workspace_path,
                        reason,
                        ..
                    } if workspace_path == &PathBuf::from(path)
                        && reason.contains("Maturation grace")
                )
            });
            assert!(retained, "{path} should be retained under maturation grace");
        }
    }

    #[test]
    fn old_and_ageless_zombies_still_prune() {
        let now_ms = 1_700_000_000_000;
        let items = young_inventory(now_ms);
        let plan = reconcile_worktree_convergence_pure(
            PathBuf::from("/repo"),
            "main",
            "sha-main-001",
            &items,
            &[],
            now_ms,
        );
        for path in ["/repo/old-zombie", "/repo/ageless-zombie"] {
            let pruned = plan.actions.iter().any(|a| {
                matches!(
                    a,
                    WorktreeConvergenceAction::PruneZombieWorkspace { workspace_path, .. }
                        if workspace_path == &PathBuf::from(path)
                )
            });
            assert!(pruned, "{path} should still prune");
        }
    }
}

/// Remove the local linked worktree and local branch of a merged agent PR.
///
/// Best-effort and bounded: the main worktree (first entry of
/// `git worktree list`) is never removed, a dirty worktree is kept, and the
/// local branch is deleted only after the remote merge was confirmed by the caller.
pub fn prune_merged_branch_worktree<R: ArgvRunner>(
    branch: &str,
    runner: &R,
) -> Result<String, String> {
    let listing_out = runner.run(&["git", "worktree", "list", "--porcelain"]);
    if !listing_out.success() {
        return Ok("local: worktree list failed, kept".to_string());
    }
    let listing = listing_out.stdout;

    // Each block: `worktree <path>` then optional `branch refs/heads/<name>`.
    let mut entries: Vec<(String, Option<String>)> = Vec::new();
    for block in listing.split("\n\n") {
        let mut path = None;
        let mut head = None;
        for line in block.lines() {
            if let Some(p) = line.strip_prefix("worktree ") {
                path = Some(p.to_string());
            } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
                head = Some(b.to_string());
            }
        }
        if let Some(p) = path {
            entries.push((p, head));
        }
    }

    let mut notes: Vec<String> = Vec::new();
    // Index 0 is the main worktree: it is never removed here.
    for (path, head) in entries.iter().skip(1) {
        if head.as_deref() != Some(branch) {
            continue;
        }
        let dirty_out = runner.run(&["git", "-C", path, "status", "--porcelain"]);
        let dirty = if dirty_out.success() {
            !dirty_out.stdout.is_empty()
        } else {
            true
        };
        if dirty {
            return Ok(format!("local: worktree {path} is dirty, kept"));
        }
        let remove_out = runner.run(&["git", "worktree", "remove", path]);
        if !remove_out.success() {
            return Ok(format!("local: worktree {path} removal failed"));
        }
        notes.push(format!("worktree {path} removed"));
    }

    let refname = format!("refs/heads/{branch}");
    let showref_out = runner.run(&["git", "show-ref", "--verify", "--quiet", &refname]);
    let exists = showref_out.success();
    if exists {
        let del_out = runner.run(&["git", "branch", "-D", branch]);
        if del_out.success() {
            notes.push(format!("local branch {branch} deleted"));
        } else {
            notes.push(format!("local branch {branch} kept (git refused)"));
        }
    }

    if notes.is_empty() {
        Ok("local: nothing to prune".to_string())
    } else {
        Ok(format!("local: {}", notes.join("; ")))
    }
}

#[cfg(test)]
mod prune_merged_branch_worktree_tests {
    use super::*;
    use crate::runner::{ArgvOutput, MapArgvRunner};

    #[test]
    fn clean_execution() {
        let branch = "agent/foo";
        let wt_path = "/repo/.worktrees/foo";
        let listing = format!(
            "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree {wt_path}\nHEAD def\nbranch refs/heads/{branch}\n"
        );
        let runner = MapArgvRunner {
            responses: vec![
                (
                    vec!["git".into(), "worktree".into(), "list".into(), "--porcelain".into()],
                    ArgvOutput { exit_code: Some(0), stdout: listing, stderr: String::new(), spawn_error: None },
                ),
                (
                    vec!["git".into(), "-C".into(), wt_path.into(), "status".into(), "--porcelain".into()],
                    ArgvOutput { exit_code: Some(0), stdout: String::new(), stderr: String::new(), spawn_error: None },
                ),
                (
                    vec!["git".into(), "worktree".into(), "remove".into(), wt_path.into()],
                    ArgvOutput { exit_code: Some(0), stdout: String::new(), stderr: String::new(), spawn_error: None },
                ),
                (
                    vec!["git".into(), "show-ref".into(), "--verify".into(), "--quiet".into(), format!("refs/heads/{branch}")],
                    ArgvOutput { exit_code: Some(0), stdout: String::new(), stderr: String::new(), spawn_error: None },
                ),
                (
                    vec!["git".into(), "branch".into(), "-D".into(), branch.into()],
                    ArgvOutput { exit_code: Some(0), stdout: String::new(), stderr: String::new(), spawn_error: None },
                ),
            ],
        };
        let result = prune_merged_branch_worktree(branch, &runner);
        assert_eq!(
            result,
            Ok(format!("local: worktree {wt_path} removed; local branch {branch} deleted"))
        );
    }
}
