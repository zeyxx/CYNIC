//! Judgment surface — read-only projection that observes the runtime's
//! current state and projects it into a single [`CynicJudgment`] through
//! the three orthogonal lenses (DIKW, SAGE, Cynefin).
//!
//! Thin surface: parse → delegate → emit. No domain policy here.

use std::process::ExitCode;

use clap::Subcommand;

mod render;
mod scan;
mod types;

#[derive(Subcommand)]
pub(crate) enum JudgmentCommand {
    /// Project the runtime's current state into a single CynicJudgment.
    #[command(name = "judgment")]
    Judgment {
        #[arg(long)]
        json: bool,
        /// Optional subject id (defaults to local hostname).
        #[arg(long)]
        subject: Option<String>,
    },
}

pub(crate) fn execute_judgment(command: JudgmentCommand) -> ExitCode {
    match command {
        JudgmentCommand::Judgment { json, subject } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let report = scan::observe_judgment(subject.as_deref());
            crate::cli_json::emit_pretty_json(&report)
        }
    }
}
