//! Pull Request management surface delegating to cynic-vcs & cynic-vcs-forgejo.

use clap::Subcommand;
use std::path::Path;
use std::process::ExitCode;

use cynic_vcs::pull_request::evaluate_pr_admission;
use cynic_vcs::pull_request::{evaluate_pr_admission, evaluate_pr_close};
use cynic_vcs_forgejo::forgejo_pr_port_from_registry;
use crate::cli_json::emit_pretty_json;

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

            let audit_report = crate::crate_audit::audit_core_crates();
            let builder_ok = audit_report.blocked_count == 0;

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
