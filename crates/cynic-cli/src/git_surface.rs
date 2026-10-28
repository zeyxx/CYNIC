use crate::cli_json::emit_pretty_json;
use clap::Subcommand;
use std::path::Path;
use std::process::ExitCode;
use cynic_lifecycle::ArgvRunner;

#[derive(Debug, Subcommand)]
pub(crate) enum GitCommand {
    /// Branch topology arms (nested noun level, no hyphen).
    Branch {
        #[command(subcommand)]
        command: GitBranchCommand,
    },
    /// Reconcile topology into actions (ReadyForPr/NeedsRebase/Archive...).
    Reconcile {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
        /// Apply safe actions (archive dead branches, gc worktrees) automatically.
        #[arg(long)]
        apply: bool,
    },
    /// Staged vs claim hygiene (git add -A detection, claim scope).
    Hygiene {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum GitBranchCommand {
    /// Branch graph (parent inference) — thin via LocalBranchGraphAdapter.
    Graph {
        #[arg(long)]
        json: bool,
    },
    /// Observe branch topology (ahead/behind/state per branch).
    Status {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_git_branch(command: GitBranchCommand) -> ExitCode {
    match command {
        GitBranchCommand::Graph { json } => crate::branch_graph_surface::execute_branch_graph(json),
        GitBranchCommand::Status { root, json } => {
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
    }
}


/// Extract branch name from a debug-formatted reconcile action.
fn extract_branch_from_action(s: &str) -> Option<String> {
    // Look for branch: "..." pattern
    let marker = "branch: \"";
    let start = s.find(marker)?;
    let rest = &s[start + marker.len()..];
    let end = rest.find('\"')?;
    Some(rest[..end].to_string())
}

pub(crate) fn execute_git(command: GitCommand) -> ExitCode {
    match command {
        GitCommand::Branch { command } => execute_git_branch(command),
        GitCommand::Reconcile { root, json, apply } => {
            let obs = cynic_host::git_topology_observer::observe_git_topology(Path::new(&root));
            let decision = cynic_host::domain::git::decision::reconcile_git_topology(&obs);
            if json {
                // Emit combined observation + decision for completeness
                #[derive(serde::Serialize)]
                struct Out<'a> {
                    observation: &'a cynic_vcs::git_topology::GitTopologyObservation,
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
                if apply {
                    println!();
                    println!("=== Applying safe actions ===");
                    let mut applied = 0;
                    let mut failed = 0;
                    for a in &decision.actions {
                        let kind_dbg = format!("{:?}", a);
                        let kind = kind_dbg.split_whitespace().next().unwrap_or("");
                        // Only apply archive and gc_worktree actions (safe, reversible)
                        if kind == "Archive" || kind == "GcWorktree" {
                            // Extract branch name from action debug string
                            // Actions are enums; we match on debug repr
                            let action_str = format!("{:?}", a);
                            // Parse branch from "Archive { branch: \"...\", ... }" or "GcWorktree { branch: \"...\", ... }"
                            if let Some(branch) = extract_branch_from_action(&action_str) {
                                let res = cynic_lifecycle::LocalArgvRunner.run(&["git", "branch", "-D", &branch]);
                                match res {
                                    o if o.success() => {
                                        println!("  ✓ archived: {}", branch);
                                        applied += 1;
                                    }
                                    o if o.spawn_error.is_none() => {
                                        let err = o.stderr;
                                        println!("  ✗ failed: {} — {}", branch, err.trim());
                                        failed += 1;
                                    }
                                    o => {
                                        let e = o.spawn_error.unwrap_or_default();
                                        println!("  ✗ error: {} — {}", branch, e);
                                        failed += 1;
                                    }
                                }
                            }
                        }
                    }
                    println!("Applied: {}, Failed: {}", applied, failed);
                }
                if !decision.unknowns.is_empty() {
                    println!("Unknowns: {:?}", decision.unknowns);
                }
                ExitCode::SUCCESS
            }
        }
        GitCommand::Hygiene { root, json } => {
            let obs = cynic_host::git_topology_observer::observe_git_hygiene(Path::new(&root));
            let decision = cynic_vcs::git_hygiene::check_hygiene(&obs);
            if json {
                #[derive(serde::Serialize)]
                struct Out<'a> {
                    observation: &'a cynic_vcs::git_hygiene::GitHygieneObservation,
                    decision: &'a cynic_vcs::git_hygiene::GitHygieneDecision,
                }
                emit_pretty_json(&Out {
                    observation: &obs,
                    decision: &decision,
                })
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
