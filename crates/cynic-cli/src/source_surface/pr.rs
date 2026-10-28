//! Pull Request management surface delegating to cynic-vcs & cynic-vcs-providers.

use clap::Subcommand;
use std::path::Path;
use std::process::ExitCode;

use cynic_vcs::pull_request::{SecretPattern, evaluate_pr_admission, evaluate_pr_close, secret_lint};
use cynic_vcs::providers::forgejo_pr_port_from_registry;
use crate::cli_json::emit_pretty_json;
use cynic_process::{ArgvRunner, LocalArgvRunner};

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
        /// Keep the head branch. Default: agent/* branches are deleted after merge.
        #[arg(long, default_value_t = false)]
        keep_branch: bool,
        #[arg(long)]
        json: bool,
    },
    /// Close an open pull request without merging.
    Close {
        #[arg(long)]
        number: u64,
        #[arg(long, default_value_t = false)]
        delete_branch: bool,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_pr(root: &Path, command: PrCommand) -> ExitCode {
    let port = match forgejo_pr_port_from_registry(root) {
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
            keep_branch,
            json,
        } => {
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

            // Builder gate: submit a build to the builder for this PR's head commit
            // and check the result. Replaces the structural audit with a real
            // compilation check.
            let builder_url = cynic_build::resolve_builder_url_with_health()
                .unwrap_or_else(|_| "http://10.10.10.119:9090".to_string());
            let builder_ok = match cynic_build::submit_workload(
                &pr.head_sha,
                None,
                "check",
                "cynic-cli-pr-merge",
                &builder_url,
                true,
            ) {
                Ok(proof) => proof.get("state").and_then(|s| s.as_str()) == Some("completed"),
                Err(e) => {
                    eprintln!("builder check failed: {e}");
                    false
                }
            };

            // Secret lint: read patterns from infra/secret-patterns.json and
            // scan the PR diff. Patterns live in a registry, not hardcoded.
            let secret_violations = match std::fs::read_to_string("infra/secret-patterns.json") {
                Ok(raw) => {
                    #[derive(serde::Deserialize)]
                    struct PatternRegistry {
                        patterns: Vec<SecretPattern>,
                    }
                    match serde_json::from_str::<PatternRegistry>(&raw) {
                        Ok(reg) => {
                            let target = "origin/main";
                            let source = &pr.head_ref;
                            let output = LocalArgvRunner.run(&["git", "diff", "--stat", &format!("{target}...{source}")]);
                            let diff = output.stdout;
                            secret_lint(&diff, &reg.patterns)
                        }
                        Err(_) => Vec::new(),
                    }
                }
                Err(_) => Vec::new(),
            };

            // Store convergence gate: Forgejo main (source) must equal the GitHub
            // projection, read live from the remotes. Local refs are not consulted:
            // they go stale between fetches and produced false blocks.
            let store_converged = match crate::source_surface::divergence::live_main_agreement(root) {
                Ok(agree) => agree,
                Err(e) => {
                    eprintln!("blocked: cannot read live main SHAs: {e}");
                    false
                }
            };

            if !store_converged {
                eprintln!("blocked: store divergence detected before merge. Converge first.");
                return ExitCode::FAILURE;
            }

            let admission = evaluate_pr_admission(pr, builder_ok, &secret_violations);

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
                match port.merge(number, &style, !keep_branch) {
                    Ok(proof) => {
                        // Local cleanup only after a confirmed remote merge of an agent branch.
                        let local = if !keep_branch && proof.head_ref.starts_with("agent/") {
                            Some(
                                cynic_vcs::source_worktree::prune_merged_branch_worktree(
                                    &proof.head_ref,
                                    &cynic_process::LocalArgvRunner,
                                )
                                .unwrap_or_else(|e| e),
                            )
                        } else {
                            None
                        };
                        if json {
                            emit_pretty_json(&proof)
                        } else {
                            println!("{}", proof.message);
                            if let Some(l) = local {
                                println!("{l}");
                            }
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
        PrCommand::Close {
            number,
            delete_branch,
            json,
        } => {
            let prs = match port.list("open") {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("blocked: cannot observe PRs before close: {e}");
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

            if let Err(blocker) = evaluate_pr_close(pr) {
                eprintln!("blocked: PR #{number} failed close admission: {blocker}");
                return ExitCode::FAILURE;
            }

            match port.close(number, delete_branch) {
                Ok(proof) => {
                    if json {
                        emit_pretty_json(&proof)
                    } else {
                        println!("{}", proof.message);
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("blocked: close failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
