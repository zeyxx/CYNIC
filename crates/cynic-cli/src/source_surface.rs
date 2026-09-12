use crate::cli_json::emit_pretty_json;
use clap::Subcommand;
use serde::Serialize;
use std::path::Path;
use std::process::ExitCode;

#[derive(Debug, Subcommand)]
pub(crate) enum SourceCommand {
    /// Observe source layer convergence status (Cortex, Forgejo, GitHub).
    Status {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
    },
    /// Run release gate checks before promoting release.
    Release {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Reconcile and synchronize source layers.
    Sync {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Reconcile, rebase, and fast-forward agent source worktrees.
    Worktree {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value = "/tmp/cynic-workspaces")]
        workspace_store: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct SourceExit {
    status: &'static str,
    summary: String,
}

pub(crate) fn execute_source(command: SourceCommand) -> ExitCode {
    match command {
        SourceCommand::Status { root, json } => {
            let path = Path::new(&root);
            let observation = cynic_host::source_sync::observe_all_layers(path);
            let decision = cynic_host::source_sync::reconcile(&observation);
            if json {
                emit_pretty_json(&decision)
            } else {
                println!("Source sync actions: {}", decision.actions.len());
                println!("Needs review: {}", decision.needs_review);
                if !decision.unknowns.is_empty() {
                    println!("Unknowns: {:?}", decision.unknowns);
                }
                ExitCode::SUCCESS
            }
        }
        SourceCommand::Release {
            root,
            dry_run,
            apply,
            json,
        } => execute_release(Path::new(&root), dry_run, apply, json),
        SourceCommand::Sync { root, apply, json } => {
            let path = Path::new(&root);
            let observation = cynic_host::source_sync::observe_all_layers(path);
            let decision = cynic_host::source_sync::reconcile(&observation);
            let plan = cynic_host::source_sync::plan(&decision);
            if apply {
                emit_blocked_source_mutation(json)
            } else {
                if json {
                    emit_pretty_json(&plan)
                } else {
                    println!(
                        "Sync plan: {} steps (dry-run, pass --apply to execute)",
                        plan.steps.len()
                    );
                    ExitCode::SUCCESS
                }
            }
        }
        SourceCommand::Worktree {
            root,
            workspace_store,
            apply,
            json,
        } => execute_worktree(Path::new(&root), Path::new(&workspace_store), apply, json),
    }
}

fn emit_blocked_source_mutation(json: bool) -> ExitCode {
    let exit = blocked_source_mutation();
    if json {
        emit_pretty_json(&exit)
    } else {
        println!("{}", exit.summary);
        ExitCode::FAILURE
    }
}

fn blocked_source_mutation() -> SourceExit {
    SourceExit {
        status: "blocked",
        summary: "blocked_unsupported_mutation: source sync and worktree apply require an admitted source-control transaction".to_string(),
    }
}

fn execute_release(root: &Path, _dry_run: bool, _apply: bool, json: bool) -> ExitCode {
    let observation = cynic_host::source_sync::observe_all_layers(root);
    let gate = cynic_host::release_bot::run_gate(root);

    let exit = if gate.all_passed() {
        SourceExit {
            status: "ready",
            summary: format!(
                "gate passed — forgejo ahead by {} commits, ready for release",
                observation.forgejo.commits_ahead.unwrap_or(0)
            ),
        }
    } else {
        SourceExit {
            status: "blocked",
            summary: format!(
                "gate failed — {} checks failed",
                gate.checks.iter().filter(|c| !c.passed).count()
            ),
        }
    };

    if json {
        emit_pretty_json(&exit)
    } else {
        println!("{}", exit.summary);
        ExitCode::SUCCESS
    }
}
fn execute_worktree(root: &Path, workspace_store: &Path, apply: bool, json: bool) -> ExitCode {
    let root_canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let ws_store_canonical = if workspace_store.exists() {
        workspace_store.to_path_buf()
    } else {
        let home_ws = std::env::var_os("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".workspace"))
            .unwrap_or_default();
        if home_ws.exists() {
            home_ws
        } else {
            workspace_store.to_path_buf()
        }
    };

    let inventory = cynic_vcs::observe_source_worktree_inventory(
        &root_canonical,
        &ws_store_canonical,
        "cynic",
        "local",
        &cynic_vcs::LocalVcsRunner,
    );
    let head_sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root_canonical)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "HEAD".to_string());

    let default_branch = "main".to_string();
    let mut snapshots = Vec::new();
    for entry in &inventory.entries {
        let is_active_root = entry.path_basis
            == cynic_protocol::SourceWorktreeInventoryPathBasis::MainRepository
            || entry.relative_path.as_deref() == Some(".")
            || entry.relative_path.as_deref() == Some("")
            || entry.candidate_id == "cynic";

        let path = if is_active_root {
            root_canonical.clone()
        } else {
            entry
                .relative_path
                .as_deref()
                .map(|r| ws_store_canonical.join(r))
                .unwrap_or_else(|| std::path::PathBuf::from(&entry.candidate_id))
        };

        let branch = entry
            .branch
            .as_deref()
            .map(|b| b.strip_prefix("refs/heads/").unwrap_or(b).to_string());

        let dirty_file_count = if path.exists() {
            std::process::Command::new("git")
                .args(["-C", &path.to_string_lossy(), "status", "--short"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
                .unwrap_or(0)
        } else {
            0
        };

        snapshots.push(cynic_protocol::source_worktree::ObservedWorktreeSnapshot {
            path,
            branch,
            head_sha: entry.head.clone().unwrap_or_default(),
            is_active_root,
            dirty_file_count,
            git_prunable: entry.git_prunable == Some(true)
                || entry.state == cynic_protocol::SourceWorktreeInventoryEntryState::Malformed,
            prunable_reason: entry.prunable_reason.clone(),
        });
    }

    let plan = cynic_protocol::source_worktree::reconcile_worktree_convergence_pure(
        root_canonical.clone(),
        &default_branch,
        &head_sha,
        &snapshots,
        &[],
    );

    if apply {
        let runner = cynic_vcs::LocalVcsRunner;
        let proof = cynic_vcs::reconcile::apply_worktree_convergence(&plan, &runner, false);

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
                    cynic_protocol::source_worktree::WorktreeConvergenceAction::PruneMergedWorkspace {
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
                    cynic_protocol::source_worktree::WorktreeConvergenceAction::EligibleForIntegration {
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
                    cynic_protocol::source_worktree::WorktreeConvergenceAction::EscalateDivergence {
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
                    cynic_protocol::source_worktree::WorktreeConvergenceAction::PruneZombieWorkspace {
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
                    cynic_protocol::source_worktree::WorktreeConvergenceAction::RetainActiveWorkspace {
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

#[cfg(test)]
mod tests {
    #[test]
    fn source_mutations_are_blocked_until_source_control_is_admitted() {
        let exit = super::blocked_source_mutation();

        assert_eq!(exit.status, "blocked");
        assert!(exit.summary.contains("blocked_unsupported_mutation"));
    }
}
