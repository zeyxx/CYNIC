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
    EnrollmentRequest {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        seed_manifest: String,
        #[arg(long)]
        machine_id: String,
        #[arg(long)]
        role: String,
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
        BootstrapFrameworkCommand::EnrollmentRequest {
            json,
            seed_manifest,
            machine_id,
            role,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let role = match framework_bootstrap::FrameworkRole::parse(&role) {
                Ok(role) => role,
                Err(error) => return fail(error),
            };
            match framework_bootstrap::build_framework_enrollment_request(
                &seed_manifest,
                &machine_id,
                role,
            ) {
                Ok(report) => emit_json(&report, true),
                Err(error) => fail(error),
            }
        }
    }
}
