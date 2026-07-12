use std::process::ExitCode;

use clap::Subcommand;

use crate::node_host_audit;
use crate::{emit_json, fail};

#[derive(Subcommand)]
pub(crate) enum AuditCommand {
    NodeHost {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 80)]
        max_commits: usize,
    },
}

pub(crate) fn execute_audit(command: AuditCommand) -> ExitCode {
    match command {
        AuditCommand::NodeHost { json, max_commits } => {
            if !json {
                return ExitCode::FAILURE;
            }
            match node_host_audit::build_node_host_audit(max_commits) {
                Ok(report) => emit_json(&report, false),
                Err(error) => fail(error),
            }
        }
    }
}
