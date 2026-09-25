use crate::cli_json::emit_pretty_json;
use clap::Subcommand;
use serde::Serialize;
use std::path::Path;
use std::process::ExitCode;

use cynic_vcs::pull_request::{PrPort, evaluate_pr_admission};
use cynic_vcs_forgejo::pull_request::ForgejoPrAdapter;

/// Read vcs_provider.sovereign_ssot from infra/registry.json and return
/// a boxed PrPort. Fails closed — returns Err if registry is unreadable or
/// provider kind is unknown.
fn pr_port_from_registry(root: &Path) -> Result<Box<dyn PrPort>, String> {
    let reg_path = root.join("infra/registry.json");
    let raw = std::fs::read_to_string(&reg_path)
        .map_err(|e| format!("registry unreadable at {}: {e}", reg_path.display()))?;
    let reg: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("registry invalid JSON: {e}"))?;

    let ssot = find_ssot_in_registry(&reg)
        .ok_or_else(|| {
            // Emit the actual keys found at /nodes to help diagnose evolution
            let nodes_len = reg.pointer("/nodes")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            format!(
                "vcs_provider.sovereign_ssot not found in registry (nodes count: {nodes_len}, \
                 tried paths: /nodes/4/source_workspace_layout/..., /nodes/0/..., /source_lifecycle/...)"
            )
        })?;

    let kind = ssot["kind"].as_str().unwrap_or("forgejo");
    let api_url = ssot["api_url"].as_str().unwrap_or("http://forgejo.synapse.cynic:3000");
    let owner = ssot["owner"].as_str().unwrap_or("jeanterre13");
    let repo = ssot["repo"].as_str().unwrap_or("CYNIC");
    let token_env = ssot["token_env"].as_str().unwrap_or("FORGEJO_TOKEN");
    let token_file = ssot["token_file"].as_str().unwrap_or("/etc/cynic/secrets/forgejo-token");

    let token = std::env::var(token_env).ok().filter(|v| !v.is_empty()).or_else(|| {
        std::fs::read_to_string(token_file)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    });

    match kind {
        "forgejo" => Ok(Box::new(ForgejoPrAdapter::new(
            api_url.to_string(),
            owner.to_string(),
            repo.to_string(),
            token,
        ))),
        other => Err(format!("unknown vcs_provider kind '{other}' — only 'forgejo' is implemented")),
    }
}

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
    /// Manage Pull Requests on the sovereign VCS provider (read from infra/registry.json).
    Pr {
        #[command(subcommand)]
        command: PrCommand,
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
                let admission =
                    cynic_host::adapters::source_admission_adapter::admit_source_sync(
                        path, &decision,
                    );
                if !admission.admitted {
                    if json {
                        emit_pretty_json(&admission)
                    } else {
                        println!("Source sync BLOCKED by admission gate.");
                        println!("Admission schema: {}", admission.schema);
                        if !admission.gates_failed.is_empty() {
                            println!("Failed gates: {:?}", admission.gates_failed);
                        }
                        for blocker in &admission.blockers {
                            println!("  blocker: {blocker}");
                        }
                        ExitCode::FAILURE
                    }
                } else {
                    let proof = cynic_host::source_sync::execute_plan(path, &plan);
                    if json {
                        emit_pretty_json(&proof)
                    } else {
                        println!(
                            "Sync execution: {} steps, success={}",
                            proof.steps_completed.len(),
                            proof.success
                        );
                        for result in &proof.steps_completed {
                            let status_sym = if result.success { "PASS" } else { "FAIL" };
                            println!("  [{status_sym}] {:?}", result.step);
                        }
                        if proof.success {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::FAILURE
                        }
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
        SourceCommand::Pr { command } => execute_pr(Path::new("."), command),
    }
}

fn execute_release(root: &Path, _dry_run: bool, apply: bool, json: bool) -> ExitCode {
    let observation = cynic_host::source_sync::observe_all_layers(root);
    let proofs = cynic_host::adapters::release_gate_adapter::collect_host_gate_proofs(root);
    let report = cynic_release::evaluate_gate_records(proofs);

    if !report.all_passed {
        let exit = SourceExit {
            status: "blocked",
            summary: format!(
                "gate failed — {} checks failed: {}",
                report.blockers.len(),
                report.blockers.join("; ")
            ),
        };
        return if json {
            emit_pretty_json(&exit)
        } else {
            println!("{}", exit.summary);
            ExitCode::FAILURE
        };
    }

    if apply {
        let remote = "github";
        let branch = "main";
        let push_result = cynic_host::adapters::release_relay_adapter::push_branch_to_remote(root, remote, branch);
        let readback = cynic_host::adapters::release_relay_adapter::verify_branch_readback(root, remote, branch);

        if json {
            emit_pretty_json(&serde_json::json!({
                "schema": "cynic.source_release_report.v1",
                "status": if push_result.is_ok() && readback.matches { "success" } else { "failed" },
                "remote": remote,
                "branch": branch,
                "readback": {
                    "local_sha": readback.local_sha,
                    "remote_sha": readback.remote_sha,
                    "matches": readback.matches,
                },
                "push_error": push_result.as_ref().err(),
            }))
        } else {
            match push_result {
                Ok(_) => {
                    println!("Release published to relay '{remote}' branch '{branch}'.");
                    println!(
                        "Read-back proof: local={} remote={} matches={}",
                        readback.local_sha.as_deref().unwrap_or("none"),
                        readback.remote_sha.as_deref().unwrap_or("none"),
                        readback.matches
                    );
                    if readback.matches {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(e) => {
                    eprintln!("Release push failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    } else {
        let exit = SourceExit {
            status: "ready",
            summary: format!(
                "gate passed — forgejo ahead by {} commits, ready for release (pass --apply to execute)",
                observation.forgejo.commits_ahead.unwrap_or(0)
            ),
        };
        if json {
            emit_pretty_json(&exit)
        } else {
            println!("{}", exit.summary);
            ExitCode::SUCCESS
        }
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

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let inventory = cynic_vcs::observe_source_worktree_inventory(
        root,
        &ws_store_canonical,
        "cynic",
        "local",
        &cynic_host::adapters::vcs_runner::LocalVcsRunner,
        now_ms,
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
            == cynic_vcs::SourceWorktreeInventoryPathBasis::MainRepository
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

        snapshots.push(cynic_vcs::source_worktree::ObservedWorktreeSnapshot {
            path,
            branch,
            head_sha: entry.head.clone().unwrap_or_default(),
            is_active_root,
            dirty_file_count,
            git_prunable: entry.git_prunable == Some(true)
                || entry.state == cynic_vcs::SourceWorktreeInventoryEntryState::Malformed,
            prunable_reason: entry.prunable_reason.clone(),
        });
    }

    let plan = cynic_vcs::source_worktree::reconcile_worktree_convergence_pure(
        root_canonical.clone(),
        &default_branch,
        &head_sha,
        &snapshots,
        &[],
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

/// Lookup the JSON pointer for the vcs_provider block,
/// trying multiple paths to be resilient to registry schema evolution.
fn find_ssot_in_registry(reg: &serde_json::Value) -> Option<&serde_json::Value> {
    // Try the node-scoped path first, then bare top-level paths
    let candidates = [
        "/nodes/4/source_workspace_layout/vcs_provider/sovereign_ssot",
        "/nodes/0/source_workspace_layout/vcs_provider/sovereign_ssot",
        "/nodes/0/source_lifecycle/vcs_provider/sovereign_ssot",
        "/source_lifecycle/vcs_provider/sovereign_ssot",
        "/vcs_provider/sovereign_ssot",
    ];
    candidates.iter().find_map(|ptr| reg.pointer(ptr))
}

#[derive(Debug, Subcommand)]
pub(crate) enum PrCommand {
    /// List open pull requests from the sovereign VCS provider.
    List {
        #[arg(long, default_value = "open")]
        state: String,
        #[arg(long)]
        json: bool,
    },
    /// Open a new pull request on the sovereign VCS provider.
    Open {
        #[arg(long)]
        title: String,
        #[arg(long)]
        head: String,
        #[arg(long, default_value = "main")]
        base: String,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long)]
        json: bool,
    },
    /// Merge an open pull request after passing all admission gates.
    Merge {
        #[arg(long)]
        number: u64,
        #[arg(long, default_value = "merge")]
        style: String,
        #[arg(long)]
        json: bool,
    },
}

fn execute_pr(root: &Path, command: PrCommand) -> ExitCode {
    let port = match pr_port_from_registry(root) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("blocked: registry error: {e}");
            return ExitCode::FAILURE;
        }
    };

    match command {
        PrCommand::List { state, json } => {
            match port.list(&state) {
                Ok(prs) => {
                    if json {
                        emit_pretty_json(&prs)
                    } else {
                        if prs.is_empty() {
                            println!("No {state} pull requests.");
                        }
                        for pr in &prs {
                            println!(
                                "  #{} [{}] {} -> {}: {}",
                                pr.number, state, pr.head_ref, pr.base_ref, pr.title
                            );
                            println!("      {}", pr.html_url);
                        }
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("blocked: pr list failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        PrCommand::Open { title, head, base, body, json } => {
            match port.open(&title, &head, &base, &body) {
                Ok(proof) => {
                    if json {
                        emit_pretty_json(&proof)
                    } else {
                        println!("PR #{} opened: {}", proof.pr_number, proof.html_url);
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("blocked: pr open failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        PrCommand::Merge { number, style, json } => {
            // Observe the PR, run admission gates, then actuate.
            let prs = match port.list("open") {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("blocked: cannot observe PRs before merge: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let pr = match prs.iter().find(|p| p.number == number) {
                Some(p) => p,
                None => {
                    eprintln!("blocked: PR #{number} not found among open PRs");
                    return ExitCode::FAILURE;
                }
            };

            // Gate evaluation — builder check proxied via cynic audit
            let builder_ok = {
                let audit_output = std::process::Command::new(
                    std::env::current_exe().unwrap_or_else(|_| "cynic".into()),
                )
                .args(["audit", "crates", "--json"])
                .output();
                match audit_output {
                    Ok(o) => o.status.success(),
                    Err(_) => false,
                }
            };

            let admission = evaluate_pr_admission(pr, builder_ok, true);

            if !admission.admitted {
                if json {
                    emit_pretty_json(&admission)
                } else {
                    eprintln!("blocked: PR #{number} failed admission gates:");
                    for b in &admission.blockers {
                        eprintln!("  - {b}");
                    }
                    ExitCode::FAILURE
                }
            } else {
                match port.merge(number, &style) {
                    Ok(proof) => {
                        if json {
                            emit_pretty_json(&proof)
                        } else {
                            println!("{}", proof.message);
                            ExitCode::SUCCESS
                        }
                    }
                    Err(e) => {
                        eprintln!("blocked: merge failed: {e}");
                        ExitCode::FAILURE
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cli_source_surface_builds() {
        let cmd = crate::SourceCommand::Status {
            root: ".".into(),
            json: false,
        };
        let _ = super::execute_source(cmd);
    }
}
