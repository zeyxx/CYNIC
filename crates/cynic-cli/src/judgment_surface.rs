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
    /// Output the content-addressed canonical digest of a judgment.
    #[command(name = "digest")]
    Digest {
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
            let report = scan::observe_judgment(subject.as_deref());
            if json {
                crate::cli_json::emit_pretty_json(&report)
            } else {
                println!("{}", render::render_judgment(&report.judgment));
                println!("Canonical Digest: {}", report.digest);
                ExitCode::SUCCESS
            }
        }
        JudgmentCommand::Digest { json, subject } => {
            let report = scan::observe_judgment(subject.as_deref());
            if json {
                let payload = serde_json::json!({
                    "schema": "cynic.judgment_digest.v1",
                    "judgment_id": report.judgment.judgment_id,
                    "digest": report.digest,
                });
                crate::cli_json::emit_pretty_json(&payload)
            } else {
                println!("{}", report.digest);
                ExitCode::SUCCESS
            }
        }
    }
}
