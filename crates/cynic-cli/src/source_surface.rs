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
                let proof = cynic_host::source_sync::execute_plan(path, &plan);
                if json {
                    emit_pretty_json(&proof)
                } else {
                    println!("Sync executed: success={}", proof.success);
                    if proof.success {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
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
    let inventory = cynic_host::source_worktree_inventory::observe_source_worktree_inventory(
        root,
        workspace_store,
        "cynic",
        "local",
        &cynic_host::source_worktree_inventory::LocalSourceWorktreeInventoryRunner,
    );

    let head_sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "HEAD".to_string());

    let plan = cynic_host::source_worktree_reconcile::reconcile_source_worktrees(
        root, &inventory, &head_sha, 0,
    );

    if apply {
        let proof = cynic_host::source_worktree_reconcile::apply_worktree_convergence(
            &plan,
            &cynic_host::update_lifecycle::LocalArgvRunner,
            false,
        );
        if json {
            emit_pretty_json(&proof)
        } else {
            println!(
                "Worktree convergence applied: {} actions, all_successful={}",
                proof.executed_actions.len(),
                proof.all_successful
            );
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
            println!(
                "Worktree convergence plan: {} actions (pass --apply to execute)",
                plan.actions.len()
            );
            println!(
                "  Fast-forward ready: {}",
                plan.summary.fast_forward_ready_count
            );
            println!(
                "  Rebase required:    {}",
                plan.summary.rebase_required_count
            );
            println!(
                "  Pruned zombies:     {}",
                plan.summary.pruned_zombies_count
            );
            println!("  Quarantined:        {}", plan.summary.quarantined_count);
            ExitCode::SUCCESS
        }
    }
}
