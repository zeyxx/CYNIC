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
    InstallCli {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        target: Option<String>,
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
        BootstrapFrameworkCommand::InstallCli {
            json,
            apply,
            source,
            target,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            match framework_bootstrap::install_standalone_cli(
                source.as_deref(),
                target.as_deref(),
                apply,
            ) {
                Ok(report) => emit_json(&report, report.status == "blocked"),
                Err(error) => fail(error),
            }
        }
    }
}
