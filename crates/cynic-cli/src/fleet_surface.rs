use std::process::ExitCode;

use clap::Subcommand;

use crate::fleet_report;
use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};

#[derive(Subcommand)]
pub(crate) enum FleetCommand {
    Report {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        proxmox_json: Option<String>,
        #[arg(long)]
        proxmox_list_text: Option<String>,
        #[arg(long)]
        watchdog_json: Option<String>,
    },
}

pub(crate) fn execute_fleet(command: FleetCommand) -> ExitCode {
    match command {
        FleetCommand::Report {
            json,
            registry,
            proxmox_json,
            proxmox_list_text,
            watchdog_json,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let runtime = resolve_runtime_paths(&RuntimeContext::from_env(registry.clone()));
            fleet_report::emit_result(fleet_report::emit_fleet_report(
                &runtime.registry_path,
                proxmox_json.as_deref(),
                proxmox_list_text.as_deref(),
                watchdog_json.as_deref(),
            ))
        }
    }
}
