use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::lab as lab_plan;

use crate::{emit_json, fail};

#[derive(Subcommand)]
pub(crate) enum LabCommand {
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
    },
    Provision {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        apply: bool,
    },
    Bundle {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        output_dir: String,
    },
    GuestBootstrap {
        #[command(subcommand)]
        command: LabGuestBootstrapCommand,
    },
    Image {
        #[command(subcommand)]
        command: LabImageCommand,
    },
}

#[derive(Subcommand)]
pub(crate) enum LabGuestBootstrapCommand {
    Bundle {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        output_dir: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum LabImageCommand {
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
    },
    Resolve {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
    },
    Fetch {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        apply: bool,
    },
}

pub(crate) fn execute_lab(command: LabCommand) -> ExitCode {
    match command {
        LabCommand::Plan { json, registry } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry = registry.unwrap_or_else(|| "infra/registries/lab.json".to_string());
            match lab_plan::build_lab_plan(&registry) {
                Ok(report) => emit_json(&report, false),
                Err(error) => fail(error),
            }
        }
        LabCommand::Provision {
            json,
            registry,
            apply,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            if !apply {
                return fail("lab provision requires --apply to make intent explicit".to_string());
            }
            let registry = registry.unwrap_or_else(|| "infra/registries/lab.json".to_string());
            match lab_plan::build_lab_provision_report(&registry, apply) {
                Ok(report) => emit_json(&report, report.status != "ready"),
                Err(error) => fail(error),
            }
        }
        LabCommand::Bundle {
            json,
            registry,
            output_dir,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry = registry.unwrap_or_else(|| "infra/registries/lab.json".to_string());
            match lab_plan::write_lab_bundle(&registry, &output_dir) {
                Ok(report) => emit_json(&report, false),
                Err(error) => fail(error),
            }
        }
        LabCommand::GuestBootstrap { command } => match command {
            LabGuestBootstrapCommand::Bundle { json, output_dir } => {
                if !json {
                    return ExitCode::FAILURE;
                }
                match lab_plan::write_guest_bootstrap_bundle(&output_dir) {
                    Ok(report) => emit_json(&report, false),
                    Err(error) => fail(error),
                }
            }
        },
        LabCommand::Image { command } => match command {
            LabImageCommand::Plan { json, registry } => {
                if !json {
                    return ExitCode::FAILURE;
                }
                let registry = registry.unwrap_or_else(|| "infra/registries/lab.json".to_string());
                match lab_plan::build_lab_image_report(&registry, false) {
                    Ok(report) => emit_json(&report, false),
                    Err(error) => fail(error),
                }
            }
            LabImageCommand::Resolve { json, registry } => {
                if !json {
                    return ExitCode::FAILURE;
                }
                let registry = registry.unwrap_or_else(|| "infra/registries/lab.json".to_string());
                match lab_plan::resolve_lab_image_sha256(&registry) {
                    Ok(report) => emit_json(&report, report.status != "resolved"),
                    Err(error) => fail(error),
                }
            }
            LabImageCommand::Fetch {
                json,
                registry,
                apply,
            } => {
                if !json {
                    return ExitCode::FAILURE;
                }
                if !apply {
                    return fail(
                        "lab image fetch requires --apply to make intent explicit".to_string(),
                    );
                }
                let registry = registry.unwrap_or_else(|| "infra/registries/lab.json".to_string());
                match lab_plan::build_lab_image_report(&registry, true) {
                    Ok(report) => emit_json(&report, report.status != "completed"),
                    Err(error) => fail(error),
                }
            }
        },
    }
}
