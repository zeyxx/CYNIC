use crate::cli_json::emit_pretty_json;
use clap::Subcommand;
use std::path::Path;
use std::process::ExitCode;

#[derive(Debug, Subcommand)]
pub(crate) enum GitCommand {
    /// Branch graph (parent inference) — thin via LocalBranchGraphAdapter.
    #[command(name = "branch-graph", aliases = ["branch_graph", "graph"])]
    BranchGraph {
        #[arg(long)]
        json: bool,
    },
    /// Observe branch topology (ahead/behind/state per branch).
    BranchStatus {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
    },
    /// Reconcile topology into actions (ReadyForPr/NeedsRebase/Archive...).
    Reconcile {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
    },
    /// Staged vs claim hygiene (git add -A detection, claim scope).
    Hygiene {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_git(command: GitCommand) -> ExitCode {
    match command {
        GitCommand::BranchGraph { json } => crate::branch_graph_surface::execute_branch_graph(json),
        GitCommand::BranchStatus { root, json } => {
            let obs = cynic_host::git_topology_observer::observe_git_topology(Path::new(&root));
            if json {
                emit_pretty_json(&obs)
            } else {
                println!(
                    "Git topology: {} branches (current: {})",
                    obs.entries.len(),
                    obs.current_branch.as_deref().unwrap_or("-")
                );
                for e in &obs.entries {
                    let ahead = e
                        .ahead
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "?".to_string());
                    let behind = e
                        .behind
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "?".to_string());
                    println!(
                        "  {:40} ahead={} behind={} state={:?} wt={:?}",
                        e.branch, ahead, behind, e.state, e.has_worktree
                    );
                }
                if !obs.observation_unknowns.is_empty() {
                    println!("Unknowns: {:?}", obs.observation_unknowns);
                }
                ExitCode::SUCCESS
            }
        }
        GitCommand::Reconcile { root, json } => {
            let obs = cynic_host::git_topology_observer::observe_git_topology(Path::new(&root));
            let decision = cynic_host::domain::git::decision::reconcile_git_topology(&obs);
            if json {
                // Emit combined observation + decision for completeness
                #[derive(serde::Serialize)]
                struct Out<'a> {
                    observation: &'a cynic_protocol::git_topology::GitTopologyObservation,
                    decision: &'a cynic_host::domain::git::decision::GitReconcileDecision,
                }
                emit_pretty_json(&Out {
                    observation: &obs,
                    decision: &decision,
                })
            } else {
                println!(
                    "Reconcile: {} actions, needs_review={}",
                    decision.actions.len(),
                    decision.needs_review
                );
                for a in &decision.actions {
                    println!("  {:?}", a);
                }
                for v in &decision.verdicts {
                    println!(
                        "  verdict {}: {:?} (stack_depth={:?})",
                        v.branch, v.verdict, v.stack_depth
                    );
                }
                if !decision.unknowns.is_empty() {
                    println!("Unknowns: {:?}", decision.unknowns);
                }
                ExitCode::SUCCESS
            }
        }
        GitCommand::Hygiene { root, json } => {
            let obs = cynic_host::git_topology_observer::observe_git_hygiene(Path::new(&root));
            let decision = cynic_protocol::git_hygiene::check_hygiene(&obs);
            if json {
                #[derive(serde::Serialize)]
                struct Out<'a> {
                    observation: &'a cynic_protocol::git_hygiene::GitHygieneObservation,
                    decision: &'a cynic_protocol::git_hygiene::GitHygieneDecision,
                }
                emit_pretty_json(&Out { observation: &obs, decision: &decision })
            } else {
                println!(
                    "Hygiene: needs_review={} violations={:?}",
                    decision.needs_review, decision.violations
                );
                println!("  staged: {:?}", obs.staged_files);
                println!("  dirty: {:?}", obs.dirty_files);
                println!("  claim: {:?} / {:?}", obs.claim_branch, obs.claim_scope);
                ExitCode::SUCCESS
            }
        }
    }
}
