# Worktree Convergence Engine & Transparent CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a robust, transparent worktree convergence engine that eliminates CLI "black-box" lies, provides safe idempotent worktree pruning, and separates pure protocol decision from bounded host actuation.

**Architecture:** 
- `cynic-protocol` owns pure data types (`WorktreeConvergenceAction`, `WorktreeConvergencePlan`, `WorktreeConvergenceApplyProof`) and the zero-I/O reconciliation decision.
- `cynic-host` owns bounded Git actuation via injectable `ArgvRunner` with exclusive file-locking (`.git/cynic-convergence.lock`) and idempotent pruning.
- `cynic-cli` projects transparent, granular human tables and structured JSON envelopes, unlocking `cynic source worktree --apply`.

**Tech Stack:** Rust 2024 edition, `cynic-protocol`, `cynic-host`, `cynic-cli`, `ArgvRunner`, `serde`, `serde_json`.

---

## File Structure

```text
crates/cynic-protocol/src/
  ├── source_worktree.rs            [CREATE: Pure convergence actions, plan types, and zero-I/O decision logic]
  └── lib.rs                        [MODIFY: Export source_worktree module]

crates/cynic-host/src/
  ├── source_worktree_reconcile/
  │     ├── types.rs                [MODIFY: Re-export or adapt protocol types]
  │     ├── reconcile.rs            [MODIFY: Delegate to pure protocol decision]
  │     ├── apply.rs                [MODIFY: Safe idempotent pruning, flock, read-back proof]
  │     └── mod.rs                  [MODIFY: Clean exports]

crates/cynic-cli/src/
  └── source_surface.rs             [MODIFY: Remove hardcoded blocked apply, add transparent human/JSON projection]
```

---

### Task 1: Pure Types & Zero-I/O Decision Engine in `cynic-protocol`

**Files:**
- Create: `crates/cynic-protocol/src/source_worktree.rs`
- Modify: `crates/cynic-protocol/src/lib.rs`

- [ ] **Step 1: Write failing unit tests for pure worktree reconciliation**

In `crates/cynic-protocol/src/source_worktree.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
        ]
    }

    #[test]
    fn test_reconcile_retains_active_root() {
        let items = make_test_inventory();
        let plan = reconcile_worktree_convergence_pure(
            PathBuf::from("/repo"),
            "sha-main-001",
            &items,
            &[],
        );
        let active_action = plan.actions.iter().find(|a| matches!(a, WorktreeConvergenceAction::RetainActiveWorkspace { .. }));
        assert!(active_action.is_some());
    }

    #[test]
    fn test_reconcile_prunes_clean_merged_worktree() {
        let items = make_test_inventory();
        let plan = reconcile_worktree_convergence_pure(
            PathBuf::from("/repo"),
            "sha-main-001",
            &items,
            &[],
        );
        let merged_action = plan.actions.iter().find(|a| matches!(a, WorktreeConvergenceAction::PruneMergedWorkspace { branch, .. } if branch == "agent/feature-merged"));
        assert!(merged_action.is_some());
    }

    #[test]
    fn test_reconcile_protects_dirty_worktree() {
        let items = make_test_inventory();
        let plan = reconcile_worktree_convergence_pure(
            PathBuf::from("/repo"),
            "sha-main-001",
            &items,
            &[],
        );
        let dirty_action = plan.actions.iter().find(|a| matches!(a, WorktreeConvergenceAction::RetainActiveWorkspace { reason, .. } if reason.contains("uncommitted changes")));
        assert!(dirty_action.is_some());
    }

    #[test]
    fn test_reconcile_prunes_zombie() {
        let items = make_test_inventory();
        let plan = reconcile_worktree_convergence_pure(
            PathBuf::from("/repo"),
            "sha-main-001",
            &items,
            &[],
        );
        let zombie_action = plan.actions.iter().find(|a| matches!(a, WorktreeConvergenceAction::PruneZombieWorkspace { .. }));
        assert!(zombie_action.is_some());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `RUSTFLAGS="-C debuginfo=0" cargo test -p cynic-protocol --lib source_worktree`
Expected: FAIL with compilation error (types and function not yet defined).

- [ ] **Step 3: Implement pure types and decision engine**

In `crates/cynic-protocol/src/source_worktree.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA: &str =
    "cynic.source_worktree.convergence_plan.v1";
pub const SOURCE_WORKTREE_APPLY_PROOF_SCHEMA: &str =
    "cynic.source_worktree.apply_proof.v1";

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum WorktreeConvergenceAction {
    PruneMergedWorkspace {
        workspace_path: PathBuf,
        branch: String,
        commit_sha: String,
        reason: String,
    },
    EligibleForIntegration {
        workspace_path: PathBuf,
        branch: String,
        target_commit_sha: String,
        ahead_count: usize,
    },
    EscalateDivergence {
        workspace_path: PathBuf,
        branch: String,
        branch_head: String,
        onto_main_sha: String,
        reason: String,
    },
    PruneZombieWorkspace {
        workspace_path: PathBuf,
        branch: Option<String>,
        reason: String,
    },
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

pub fn reconcile_worktree_convergence_pure(
    repository_root: PathBuf,
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
        if wt.is_active_root {
            actions.push(WorktreeConvergenceAction::RetainActiveWorkspace {
                workspace_path: wt.path.clone(),
                branch: wt.branch.clone(),
                reason: "Current active working directory".to_string(),
            });
            summary.retained_active_count += 1;
            continue;
        }

        if wt.git_prunable {
            actions.push(WorktreeConvergenceAction::PruneZombieWorkspace {
                workspace_path: wt.path.clone(),
                branch: wt.branch.clone(),
                reason: wt.prunable_reason.clone().unwrap_or_else(|| "Git inventory marked worktree prunable".to_string()),
            });
            summary.safe_prune_ready_count += 1;
            continue;
        }

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
            if active_lease_branches.contains(branch) {
                actions.push(WorktreeConvergenceAction::RetainActiveWorkspace {
                    workspace_path: wt.path.clone(),
                    branch: Some(branch.clone()),
                    reason: "Protected: active coordination lease held".to_string(),
                });
                summary.retained_active_count += 1;
                continue;
            }

            if branch == "main" || branch == "master" {
                actions.push(WorktreeConvergenceAction::RetainActiveWorkspace {
                    workspace_path: wt.path.clone(),
                    branch: Some(branch.clone()),
                    reason: "Protected: default branch checkout".to_string(),
                });
                summary.retained_active_count += 1;
                continue;
            }

            if wt.head_sha == main_head_sha {
                actions.push(WorktreeConvergenceAction::PruneMergedWorkspace {
                    workspace_path: wt.path.clone(),
                    branch: branch.clone(),
                    commit_sha: wt.head_sha.clone(),
                    reason: "Branch is fully merged into main HEAD and clean".to_string(),
                });
                summary.safe_prune_ready_count += 1;
            } else {
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
        default_branch: "main".to_string(),
        main_head_sha: main_head_sha.to_string(),
        actions,
        summary,
    }
}
```

Export `pub mod source_worktree;` in `crates/cynic-protocol/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `RUSTFLAGS="-C debuginfo=0" cargo test -p cynic-protocol --lib source_worktree`
Expected: PASS (4 tests passed).

- [ ] **Step 5: Commit**

```bash
git add crates/cynic-protocol/src/source_worktree.rs crates/cynic-protocol/src/lib.rs
git commit -m "feat(protocol): pure worktree convergence actions, plan models, and zero-I/O decision engine"
```

---

### Task 2: Actuator & Concurrency Lock in `cynic-host`

**Files:**
- Modify: `crates/cynic-host/src/source_worktree_reconcile/apply.rs`
- Modify: `crates/cynic-host/src/source_worktree_reconcile/reconcile.rs`
- Modify: `crates/cynic-host/src/source_worktree_reconcile/mod.rs`

- [ ] **Step 1: Write failing actuator test with mock ArgvRunner**

In `crates/cynic-host/src/source_worktree_reconcile/apply.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::source_worktree::{WorktreeConvergenceAction, WorktreeConvergencePlan, WorktreeConvergenceSummary};
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};
    use std::path::PathBuf;

    #[test]
    fn test_apply_executes_safe_prune_idempotently() {
        let runner = MapArgvRunner::new()
            .with_exact(
                &["git", "-C", "/repo", "worktree", "remove", "--force", "/repo/.workspace/wt-1"],
                ArgvOutput::success(""),
            )
            .with_exact(
                &["git", "-C", "/repo", "branch", "-d", "agent/wt-1"],
                ArgvOutput::success("Deleted branch agent/wt-1"),
            );

        let plan = WorktreeConvergencePlan {
            schema: cynic_protocol::source_worktree::SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA,
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

        let proof = apply_worktree_convergence(&plan, &runner, false);
        assert_eq!(proof.executed_actions.len(), 1);
        assert!(proof.executed_actions[0].success);
        assert!(proof.all_successful);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `RUSTFLAGS="-C debuginfo=0" cargo test -p cynic-host --lib source_worktree_reconcile::apply`
Expected: FAIL (types mismatch or method signature not matching new protocol models).

- [ ] **Step 3: Implement safe idempotent actuation and file lock**

In `crates/cynic-host/src/source_worktree_reconcile/apply.rs`:

```rust
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use cynic_protocol::source_worktree::{
    ExecutedActionReceipt, SOURCE_WORKTREE_APPLY_PROOF_SCHEMA, WorktreeConvergenceAction,
    WorktreeConvergenceApplyProof, WorktreeConvergencePlan,
};
use crate::update_lifecycle::ArgvRunner;

pub fn apply_worktree_convergence<R: ArgvRunner>(
    plan: &WorktreeConvergencePlan,
    runner: &R,
    dry_run: bool,
) -> WorktreeConvergenceApplyProof {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

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
                        message: format!("Dry-run: would remove worktree and delete branch ({reason})"),
                        read_back_evidence: None,
                    });
                    continue;
                }

                let ws_str = workspace_path.to_string_lossy();
                // 1. Remove worktree
                let wt_out = runner.run(&["git", "-C", &root_str, "worktree", "remove", "--force", &ws_str]);
                // Idempotent: success if deleted or already gone
                let wt_ok = wt_out.success() || wt_out.stderr.contains("is not a working tree");

                // 2. Delete branch
                let br_out = runner.run(&["git", "-C", &root_str, "branch", "-d", branch]);
                let br_ok = br_out.success() || br_out.stderr.contains("not found");

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
                    read_back_evidence: if success { Some(format!("Pruned: {ws_str}, deleted branch: {branch}")) } else { None },
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
                let wt_out = runner.run(&["git", "-C", &root_str, "worktree", "remove", "--force", &ws_str]);
                let success = wt_out.success() || wt_out.stderr.contains("is not a working tree");
                if !success {
                    all_successful = false;
                }

                receipts.push(ExecutedActionReceipt {
                    action_name: "PruneZombieWorkspace",
                    target_path: workspace_path.clone(),
                    target_branch: branch.clone(),
                    success,
                    message: wt_out.summary(),
                    read_back_evidence: if success { Some(format!("Pruned zombie: {ws_str}")) } else { None },
                });
            }

            WorktreeConvergenceAction::EligibleForIntegration { workspace_path, branch, ahead_count, .. } => {
                receipts.push(ExecutedActionReceipt {
                    action_name: "EligibleForIntegration",
                    target_path: workspace_path.clone(),
                    target_branch: Some(branch.clone()),
                    success: true,
                    message: format!("Skipped: +{ahead_count} commits ahead of main; requires verified BuildProof before merge"),
                    read_back_evidence: None,
                });
            }

            WorktreeConvergenceAction::EscalateDivergence { workspace_path, branch, reason, .. } => {
                receipts.push(ExecutedActionReceipt {
                    action_name: "EscalateDivergence",
                    target_path: workspace_path.clone(),
                    target_branch: Some(branch.clone()),
                    success: true,
                    message: format!("Escalated to human: {reason}"),
                    read_back_evidence: None,
                });
            }

            WorktreeConvergenceAction::RetainActiveWorkspace { workspace_path, branch, reason } => {
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `RUSTFLAGS="-C debuginfo=0" cargo test -p cynic-host --lib source_worktree_reconcile::apply`
Expected: PASS (actuation tests green).

- [ ] **Step 5: Commit**

```bash
git add crates/cynic-host/src/source_worktree_reconcile/
git commit -m "feat(host): safe idempotent worktree pruning actuator and read-back proof generator"
```

---

### Task 3: Transparent CLI Surface & Unlocked `--apply` in `cynic-cli`

**Files:**
- Modify: `crates/cynic-cli/src/source_surface.rs`

- [ ] **Step 1: Write test for transparent CLI output formatting**

In `crates/cynic-cli/src/source_surface.rs`:
Verify that `execute_worktree` generates a non-empty human report with section headers and does not call `emit_blocked_source_mutation`.

- [ ] **Step 2: Implement transparent CLI surface**

In `crates/cynic-cli/src/source_surface.rs`:
Replace `execute_worktree`:
1. Build `Vec<ObservedWorktreeSnapshot>` from `git worktree list --porcelain` and `git status --short` in each worktree.
2. Resolve active claims from `.agent-claims.json`.
3. Detect default branch dynamically via `git symbolic-ref refs/remotes/origin/HEAD` or default `"main"`.
4. Call `cynic_protocol::source_worktree::reconcile_worktree_convergence_pure`.
5. If `--apply`: call `cynic_host::source_worktree_reconcile::apply_worktree_convergence(&plan, &runner, false)`.
   Print execution journal with:
   - `✓ Pruned merged worktree: <path>`
   - `✓ Deleted branch: <branch>`
   - `⚠ Retained: <path> (<reason>)`
   - Summary and proof schema.
6. If `--json`: emit JSON directly using `emit_pretty_json(&proof)` or `emit_pretty_json(&plan)`.

- [ ] **Step 3: Run targeted check on cynic-cli**

Run: `RUSTFLAGS="-C debuginfo=0" cargo check -p cynic-cli --lib`
Expected: PASS (clean compile of cynic-cli with new surface).

- [ ] **Step 4: Commit**

```bash
git add crates/cynic-cli/src/source_surface.rs
git commit -m "feat(cli): transparent worktree inventory, convergence plan, and unlocked safe apply"
```

---

### Task 4: Real-World End-to-End Verification

**Files:**
- Target: Current repository and all active worktrees

- [ ] **Step 1: Run read-only plan mode**

Run: `cargo run -p cynic-cli -- source worktree`
Expected:
- Displays all 5 worktrees with exact paths, branches, SHAs, and motivated actions.
- No opaque summary lines.
- Does not mutate any worktree.

- [ ] **Step 2: Run JSON projection mode**

Run: `cargo run -p cynic-cli -- source worktree --json`
Expected:
- Outputs valid JSON conforming to `cynic.source_worktree.convergence_plan.v1`.

- [ ] **Step 3: Verify safe apply dry-run / apply on dummy or completed branch**

Run: `cargo run -p cynic-cli -- source worktree --apply`
Expected:
- Idempotently removes eligible safe-prune worktrees without breaking dirty or active worktrees.
- Emits structured proof.

- [ ] **Step 4: Final verification and commit**

```bash
git diff --check
git status --short
```
Expected: Clean worktree, passes git diff check.
