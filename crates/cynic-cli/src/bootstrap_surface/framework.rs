use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::framework_bootstrap;

use crate::{emit_json, fail};

#[derive(Subcommand)]
pub(crate) enum BootstrapFrameworkCommand {
    Bundle {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        output_dir: String,
    },
}

pub(crate) fn execute_framework(command: BootstrapFrameworkCommand) -> ExitCode {
    match command {
        BootstrapFrameworkCommand::Bundle { json, output_dir } => {
            if !json {
                return ExitCode::FAILURE;
            }
            match framework_bootstrap::write_framework_bootstrap_bundle(&output_dir) {
                Ok(report) => emit_json(&report, report.status != "written"),
                Err(error) => fail(error),
            }
        }
    }
}
