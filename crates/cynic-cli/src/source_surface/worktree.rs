//! Worktree convergence execution delegating to cynic-vcs & cynic-host.

use std::path::Path;
use std::process::ExitCode;

use crate::cli_json::emit_pretty_json;

pub(crate) fn execute_worktree(root: &Path, workspace_store: &Path, apply: bool, json: bool) -> ExitCode {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let runner = cynic_host::adapters::vcs_runner::LocalVcsRunner;
    let (head_sha, snapshots) =
        cynic_vcs::source_worktree_cli::observe_worktree_convergence_snapshots(
            root,
            workspace_store,
            &runner,
            now_ms,
        );
    let root_canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let default_branch = "main".to_string();
    let plan = cynic_vcs::source_worktree_cli::plan_worktree_convergence(
        root_canonical,
        default_branch.clone(),
        head_sha.clone(),
        &snapshots,
    );

    if apply {
        let runner = cynic_host::adapters::vcs_runner::LocalVcsRunner;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let proof = cynic_vcs::reconcile::apply_worktree_convergence(&plan, &runner, false, now);

        if json {
            emit_pretty_json(&proof)
        } else {
            println!("=== CYNIC Worktree Convergence Execution Journal ===");
            println!("Repository: {}", root.display());
            println!("Main HEAD:  {}", head_sha);
            println!("Actions Executed ({}):", proof.executed_actions.len());
            for (idx, action) in proof.executed_actions.iter().enumerate() {
                let status_sym = if action.success { "✓" } else { "✗" };
                let branch_str = action.target_branch.as_deref().unwrap_or("<detached>");
                println!(
                    "  [{}] {} {} ({})",
                    idx + 1,
                    status_sym,
                    action.action_name,
                    branch_str
                );
                println!("      Path:    {}", action.target_path.display());
                println!("      Details: {}", action.message);
                if let Some(evidence) = &action.read_back_evidence {
                    println!("      Proof:   {}", evidence);
                }
            }
            println!();
            println!(
                "Apply Status: {}",
                if proof.all_successful {
                    "SUCCESS (all actions executed cleanly)"
                } else {
                    "PARTIAL (some actions failed, check details above)"
                }
            );
            println!("Proof Schema: {}", proof.schema);
            if proof.all_successful {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    } else {
        if json {
            emit_pretty_json(&plan)
        } else {
            println!("=== CYNIC Worktree Inventory & Convergence Plan ===");
            println!("Repository Root:  {}", root.display());
            println!("Default Branch:   {} ({})", default_branch, head_sha);
            println!("Observed Worktrees ({}):", plan.summary.total_observed);

            for (idx, action) in plan.actions.iter().enumerate() {
                match action {
                    cynic_vcs::source_worktree::WorktreeConvergenceAction::PruneMergedWorkspace {
                        workspace_path,
                        branch,
                        reason,
                        ..
                    } => {
                        println!("  [{}] {} (Ready for Safe Prune)", idx + 1, branch);
                        println!("      Path:     {}", workspace_path.display());
                        println!("      Decision: PruneMergedWorkspace");
                        println!("      Reason:   {}", reason);
                    }
                    cynic_vcs::source_worktree::WorktreeConvergenceAction::EligibleForIntegration {
                        workspace_path,
                        branch,
                        ahead_count,
                        ..
                    } => {
                        println!("  [{}] {} (+{} ahead of main)", idx + 1, branch, ahead_count);
                        println!("      Path:     {}", workspace_path.display());
                        println!("      Decision: EligibleForIntegration");
                        println!("      Reason:   Strictly ahead; requires verified BuildProof before merge");
                    }
                    cynic_vcs::source_worktree::WorktreeConvergenceAction::EscalateDivergence {
                        workspace_path,
                        branch,
                        reason,
                        ..
                    } => {
                        println!("  [{}] {} (Diverged)", idx + 1, branch);
                        println!("      Path:     {}", workspace_path.display());
                        println!("      Decision: EscalateDivergence");
                        println!("      Reason:   {}", reason);
                    }
                    cynic_vcs::source_worktree::WorktreeConvergenceAction::PruneZombieWorkspace {
                        workspace_path,
                        branch,
                        reason,
                    } => {
                        let b_name = branch.as_deref().unwrap_or("<detached zombie>");
                        println!("  [{}] {} (Zombie / Prunable)", idx + 1, b_name);
                        println!("      Path:     {}", workspace_path.display());
                        println!("      Decision: PruneZombieWorkspace");
                        println!("      Reason:   {}", reason);
                    }
                    cynic_vcs::source_worktree::WorktreeConvergenceAction::RetainActiveWorkspace {
                        workspace_path,
                        branch,
                        reason,
                    } => {
                        let b_name = branch.as_deref().unwrap_or("<detached>");
                        println!("  [{}] {} (Retained)", idx + 1, b_name);
                        println!("      Path:     {}", workspace_path.display());
                        println!("      Decision: RetainActiveWorkspace");
                        println!("      Reason:   {}", reason);
                    }
                }
            }

            println!();
            println!(
                "Summary: {} safe to prune, {} integration eligible, {} retained, {} escalations required.",
                plan.summary.safe_prune_ready_count,
                plan.summary.integration_eligible_count,
                plan.summary.retained_active_count,
                plan.summary.escalation_required_count,
            );
            if plan.summary.safe_prune_ready_count > 0 {
                println!(
                    "To apply safe pruning actions, rerun with: cynic source worktree --apply"
                );
            } else {
                println!("No safe pruning actions pending.");
            }
            ExitCode::SUCCESS
        }
    }
}
