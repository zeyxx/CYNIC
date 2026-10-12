use crate::cli_json::emit_pretty_json;
use clap::Subcommand;
use serde::Serialize;
use std::path::Path;
use std::process::ExitCode;

use cynic_vcs::pull_request::{PrPort, evaluate_pr_admission};
use cynic_vcs_forgejo::forgejo_pr_port_from_registry;

mod release_deployment_cli;
mod sync_coherence;
mod worktree_allocation;

use release_deployment_cli::execute_deploy;
use worktree_allocation::execute_allocate_worktree;

/// Read vcs_provider.sovereign_ssot from infra/registry.json and return
/// a boxed PrPort. Fails closed — returns Err if registry is unreadable or
/// provider kind is unknown.
///
/// The registry read, SSOT lookup, token resolution (environment →
/// principal file → OS keyring) and adapter construction all live in
/// `cynic-vcs-forgejo`; this surface only delegates.
fn pr_port_from_registry(root: &Path) -> Result<Box<dyn PrPort>, String> {
    forgejo_pr_port_from_registry(root)
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
        /// Allocate an isolated agent worktree for the given branch name
        #[arg(long)]
        allocate: Option<String>,
        /// Base ref for isolated worktree allocation (defaults to origin/main)
        #[arg(long)]
        base: Option<String>,
        /// Explicit target path for allocation (defaults to ~/.cynic/worktrees/<branch>)
        #[arg(long)]
        path: Option<String>,
    },
    /// Manage Pull Requests on the sovereign VCS provider (read from infra/registry.json).
    Pr {
        #[command(subcommand)]
        command: PrCommand,
    },
    /// Deploy built release binaries atomically to /var/lib/cynic/releases/<version>.
    ///
    /// Without --restart-unit the release is STAGED only (files placed,
    /// symlink switched, no daemon touched). With --restart-unit (repeatable
    /// `unit[:health_url]`, e.g. `cynic-kernel.service:http://127.0.0.1:3030/health`)
    /// each declared unit is restarted, health-probed, and the deploy rolls
    /// back on failure. Only declared units are ever restarted.
    Deploy {
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        /// Target workspace directory (defaults to current dir)
        #[arg(long, default_value = ".")]
        root: String,
        /// Explicit version override (defaults to Cargo.toml package version)
        #[arg(long)]
        version: Option<String>,
        /// Actually perform deployment (otherwise dry-run / planned)
        #[arg(long)]
        apply: bool,
        /// Emit JSON report
        #[arg(long)]
        json: bool,
        /// Declared unit to restart after switch, `unit[:health_url]`.
        /// Repeatable. Without it the release stays staged.
        #[arg(long)]
        restart_unit: Vec<String>,
    },
    /// Plan the public projection of the private tree from contracts/publication.json.
    ///
    /// Read-only: emits a plan of which paths are published, which are excluded,
    /// which are undeclared, and which private literals no redaction covers. It
    /// performs no mutation and never pushes. Admission is refused on any
    /// undeclared path, dead rule, or uncovered private literal.
    Publish {
        #[arg(long, default_value = ".")]
        root: String,
        /// Emit JSON plan
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct SourceExit {
    status: &'static str,
    summary: String,
}

/// Read the proof log and reduce it to the builder's verdict on one commit.
///
/// The MOST RECENT proof for the commit is authoritative: a commit that was
/// blocked and later fixed must be releasable, and one that converged and was
/// then re-triggered into failure must not be. Reading the newest line is what
/// makes a fix actually count.
///
/// An unreadable log is `Unreadable`, never `NeverTested`: evidence exists and
/// cannot be read, which is a different problem with a different fix.
fn observe_build_proof(sha: &str) -> cynic_release::build_proof::BuildProofObservation {
    use cynic_release::build_proof::BuildProofObservation;

    let log_path = match crate::build_proof_log::default_proof_log_path() {
        Ok(path) => path,
        // No HOME, no log: nothing was ever retained for this commit.
        Err(_) => return BuildProofObservation::NeverTested,
    };
    let (proofs, skipped) = match crate::build_proof_log::read_proofs(&log_path, Some(sha), 0) {
        Ok(reading) => reading,
        Err(_) => return BuildProofObservation::NeverTested,
    };
    cynic_release::reduce_newest_proof(&proofs, skipped.len())
}

pub(crate) fn execute_source(command: SourceCommand) -> ExitCode {
    match command {
        SourceCommand::Publish { root, json } => execute_publish(Path::new(&root), json),
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
                let admission = cynic_host::adapters::source_admission_adapter::admit_source_sync(
                    path, &decision,
                );
                if !admission.admitted {
                    let coherence = sync_coherence::project_sync_coherence_vector(
                        &observation,
                        &decision,
                        None,
                    );
                    let report = sync_coherence::SourceSyncReport {
                        schema: sync_coherence::SOURCE_SYNC_REPORT_SCHEMA,
                        coherence,
                        plan: Some(plan),
                        proof: None,
                    };
                    if json {
                        emit_pretty_json(&report)
                    } else {
                        println!("Source sync BLOCKED by admission gate.");
                        println!("Status:   {:?}", report.coherence.status);
                        println!(
                            "Compass:  {}",
                            report.coherence.compass.desired_state_digest
                        );
                        println!("Blockers: {:?}", report.coherence.blockers);
                        ExitCode::FAILURE
                    }
                } else {
                    let proof = cynic_host::source_sync::execute_plan(path, &plan);
                    let read_back = if proof.success {
                        observation.cortex.head_sha.as_deref()
                    } else {
                        None
                    };
                    let coherence = sync_coherence::project_sync_coherence_vector(
                        &observation,
                        &decision,
                        read_back,
                    );
                    let report = sync_coherence::SourceSyncReport {
                        schema: sync_coherence::SOURCE_SYNC_REPORT_SCHEMA,
                        coherence,
                        plan: Some(plan),
                        proof: Some(proof.clone()),
                    };
                    if json {
                        emit_pretty_json(&report)
                    } else {
                        println!(
                            "Sync execution: {} steps, success={}",
                            proof.steps_completed.len(),
                            proof.success
                        );
                        println!("Status:   {:?}", report.coherence.status);
                        println!(
                            "Compass:  {}",
                            report.coherence.compass.desired_state_digest
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
                let coherence =
                    sync_coherence::project_sync_coherence_vector(&observation, &decision, None);
                let report = sync_coherence::SourceSyncReport {
                    schema: sync_coherence::SOURCE_SYNC_REPORT_SCHEMA,
                    coherence,
                    plan: Some(plan.clone()),
                    proof: None,
                };
                if json {
                    emit_pretty_json(&report)
                } else {
                    println!("=== CYNIC Source Coherence Vector ===");
                    println!("  Subject:    {}", report.coherence.compass.subject_id);
                    println!("  Vertical:   {}", report.coherence.compass.vertical_id);
                    println!("  Status:     {:?}", report.coherence.status);
                    println!(
                        "  Compass:    {}",
                        report.coherence.compass.desired_state_digest
                    );
                    println!("  Decision:   {}", report.coherence.vector.decision);
                    if let Some(act) = &report.coherence.vector.action {
                        println!("  Action:     {}", act);
                    }
                    if !report.coherence.blockers.is_empty() {
                        println!("  Blockers:   {:?}", report.coherence.blockers);
                    }
                    if !report.coherence.residual_unknowns.is_empty() {
                        println!("  Unknowns:   {:?}", report.coherence.residual_unknowns);
                    }
                    println!(
                        "\nSync plan: {} steps (dry-run, pass --apply to execute)",
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
            allocate,
            base,
            path,
        } => {
            if let Some(branch) = allocate {
                execute_allocate_worktree(
                    Path::new(&root),
                    Path::new(&workspace_store),
                    &branch,
                    base.as_deref(),
                    path.as_deref().map(Path::new),
                    json,
                )
            } else {
                execute_worktree(Path::new(&root), Path::new(&workspace_store), apply, json)
            }
        }
        SourceCommand::Pr { command } => execute_pr(Path::new("."), command),
        SourceCommand::Deploy {
            state_root,
            root,
            version,
            apply,
            json,
            restart_unit,
        } => execute_deploy(
            Path::new(&root),
            Path::new(&state_root),
            version.as_deref(),
            apply,
            json,
            &restart_unit,
        ),
    }
}

fn execute_release(root: &Path, _dry_run: bool, apply: bool, json: bool) -> ExitCode {
    let observation = cynic_host::source_sync::observe_all_layers(root);
    let mut proofs = cynic_release::adapters::release_gate_adapter::collect_host_gate_proofs(root);

    // The builder's verdict on THIS commit, read from the proof log the trigger
    // writes. `contracts/release-gates.json` declares `build_proof` mandatory:
    // the builder is the declared SSOT for build validation, so a commit never
    // validated there has no evidence to promote on.
    //
    // Four outcomes, three of them blocking — Converged, Blocked, NeverTested,
    // Unreadable — and they are deliberately kept distinct: "not validated" and
    // "evidence unreadable" call for different operator action.
    let head_sha = cynic_vcs::observe_head_sha(root);
    let build_observation = match head_sha.as_deref() {
        Some(sha) => observe_build_proof(sha),
        // HEAD unreadable: no commit to look up, so no proof exists for it.
        None => cynic_release::build_proof::BuildProofObservation::NeverTested,
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    proofs.push(build_observation.to_gate_record(now));

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
            emit_pretty_json(&serde_json::json!({
                "schema": "cynic.source_release_report.v1",
                "status": "blocked",
                "commit": head_sha,
                "build_proof": build_observation,
                "gate_records": report.records,
                "blockers": report.blockers,
            }))
        } else {
            println!("{}", exit.summary);
            ExitCode::FAILURE
        };
    }

    if apply {
        let remote = "github";
        let branch = "main";
        let push_result = cynic_release::adapters::release_relay_adapter::push_branch_to_remote(
            root, remote, branch,
        );
        let readback = cynic_release::adapters::release_relay_adapter::verify_branch_readback(
            root, remote, branch,
        );

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
        default_branch,
        head_sha,
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
        PrCommand::List { state, json } => match port.list(&state) {
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
        },
        PrCommand::Open {
            title,
            head,
            base,
            body,
            json,
        } => match port.open(&title, &head, &base, &body) {
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
        },
        PrCommand::Merge {
            number,
            style,
            json,
        } => {
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

            // Gate evaluation — builder check proxied via cynic audit.
            // NOTE: the subcommand is `audit crate` (singular). A past
            // rename left this gate invoking a dead `audit crates` name,
            // which failed closed on every merge (observed 2026-10-07).
            let builder_ok = {
                let audit_output = std::process::Command::new(
                    std::env::current_exe().unwrap_or_else(|_| "cynic".into()),
                )
                .args(["audit", "crate", "--json"])
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

/// Plan the public projection of the private tree.
///
/// Read-only. Observation is `git ls-files -z` (unquoted, so non-ASCII paths
/// survive) plus the bytes of every path the contract publishes. The pure
/// decision lives in `cynic_vcs::plan_publication`; this function only reads the
/// substrate and projects the plan. It never writes and never pushes.
fn execute_publish(root: &Path, json: bool) -> ExitCode {
    let contract_path = root.join("contracts/publication.json");
    let raw = match std::fs::read_to_string(&contract_path) {
        Ok(raw) => raw,
        Err(e) => {
            eprintln!("blocked: cannot read {}: {e}", contract_path.display());
            return ExitCode::FAILURE;
        }
    };
    let contract = match cynic_vcs::PublicationContract::from_json(&raw) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("blocked: {e}");
            return ExitCode::FAILURE;
        }
    };

    let runner = cynic_host::adapters::vcs_runner::LocalVcsRunner;
    let (tracked, files) = match cynic_vcs::source_worktree_cli::observe_publication_inputs(
        root, &contract, &runner,
    ) {
        Ok(inputs) => inputs,
        Err(e) => {
            eprintln!("blocked: {e}");
            return ExitCode::FAILURE;
        }
    };

    let plan = cynic_vcs::plan_publication(&contract, &tracked, &files);
    let admitted = plan.is_admitted();

    if json {
        emit_pretty_json(&plan);
        return if admitted {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    println!(
        "Publication plan — {} ({})",
        plan.contract_schema, plan.contract_status
    );
    println!("  tracked:   {}", plan.tracked_count);
    println!("  published: {}", plan.published_count);
    println!("  excluded:  {}", plan.excluded_count);
    println!("  undeclared: {}", plan.undeclared.len());
    println!("  dead rules: {}", plan.dead_rules.len());
    println!(
        "  uncovered private literals: {}",
        plan.uncovered_literals.len()
    );
    for literal in plan.uncovered_literals.iter().take(10) {
        println!("    {} ({} path(s))", literal.literal, literal.paths.len());
    }
    if admitted {
        println!("ADMITTED — the private tree satisfies contracts/publication.json.");
        ExitCode::SUCCESS
    } else {
        println!("REFUSED — {} blocker(s):", plan.blockers.len());
        for blocker in plan.blockers.iter().take(20) {
            println!("    {blocker}");
        }
        if plan.blockers.len() > 20 {
            println!("    … and {} more", plan.blockers.len() - 20);
        }
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cli_source_surface_builds() {
        let cmd = super::SourceCommand::Status {
            root: ".".into(),
            json: false,
        };
        let _ = super::execute_source(cmd);
    }
}
