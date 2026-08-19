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
    /// Discover declared nodes and probe their services.
    Discover {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
    },
    /// Aggregate fleet health from heartbeats.
    Health {
        #[arg(long, default_value = "30")]
        heartbeat_timeout_s: u64,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_fleet(command: FleetCommand) -> ExitCode {
    match command {
        FleetCommand::Report { json, registry, proxmox_json, proxmox_list_text, watchdog_json } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let runtime = resolve_runtime_paths(&RuntimeContext::from_env(registry));
            fleet_report::emit_result(fleet_report::emit_fleet_report(
                &runtime.registry_path,
                proxmox_json.as_deref(),
                proxmox_list_text.as_deref(),
                watchdog_json.as_deref(),
            ))
        }
        FleetCommand::Discover { json, registry } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path = resolve_runtime_paths(&RuntimeContext::from_env(registry)).registry_path;
            let adapter = cynic_host::fleet_discovery::FleetDiscoveryAdapter::new(
                registry_path.to_string_lossy().to_string(),
            );
            let report = adapter.discover();
            crate::cli_json::emit_pretty_json(&report)
        }
        FleetCommand::Health { heartbeat_timeout_s, json } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(None)).registry_path;
            let adapter = cynic_host::fleet_health_observer::FleetHealthObserver::new(
                registry_path.to_string_lossy().to_string(),
                heartbeat_timeout_s,
            );
            let report = adapter.observe();
            crate::cli_json::emit_pretty_json(&report)
        }
}
