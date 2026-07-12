use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
mod audit_surface;
mod bootstrap_surface;
mod command_manifest;
mod dev_surface;
mod fleet_observation;
mod fleet_report;
mod fleet_surface;
mod host_exec_alias_surface;
mod lab_surface;
mod network_surface;
mod node_host_audit;
mod runtime_paths;
mod runtime_suite;
mod runtime_surface;
mod surface_audit;
mod surface_registry;
mod transport_surface;
use audit_surface::{AuditCommand, execute_audit};
use bootstrap_surface::{BootstrapCommand, execute_bootstrap};
use command_manifest::commands_manifest;
use dev_surface::{DevCommand, execute_dev};
use fleet_surface::{FleetCommand, execute_fleet};
use host_exec_alias_surface::maybe_run_host_exec_alias;
use lab_surface::{LabCommand, execute_lab};
use network_surface::{NetworkCommand, execute_network};
use runtime_paths::{RuntimeContext, resolve_runtime_paths};
use runtime_surface::{RuntimeCommand, execute_runtime};
use serde::Serialize;
use transport_surface::{TransportCommand, execute_transport};

#[derive(Parser)]
#[command(name = "cynic", about = "Portable CYNIC operator CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Commands {
        #[arg(long)]
        json: bool,
    },
    Fleet {
        #[command(subcommand)]
        command: FleetCommand,
    },
    Runtime {
        #[command(subcommand)]
        command: RuntimeCommand,
    },
    Dev {
        #[command(subcommand)]
        command: DevCommand,
    },
    Bootstrap {
        #[command(subcommand)]
        command: BootstrapCommand,
    },
    Lab {
        #[command(subcommand)]
        command: LabCommand,
    },
    Transport {
        #[command(subcommand)]
        command: TransportCommand,
    },
    Network {
        #[command(subcommand)]
        command: NetworkCommand,
    },
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
}

fn main() -> ExitCode {
    if let Some(exit_code) = maybe_run_host_exec_alias() {
        return exit_code;
    }

    let cli = Cli::parse();
    match cli.command {
        None => {
            let mut command = Cli::command();
            match command.print_help() {
                Ok(()) => {
                    println!();
                    ExitCode::SUCCESS
                }
                Err(_) => ExitCode::FAILURE,
            }
        }
        Some(Command::Commands { json }) => {
            if !json {
                return ExitCode::FAILURE;
            }
            emit_json(&commands_manifest(), false)
        }
        Some(Command::Fleet { command }) => execute_fleet(command),
        Some(Command::Runtime { command }) => execute_runtime(command),
        Some(Command::Dev { command }) => execute_dev(command),
        Some(Command::Lab { command }) => execute_lab(command),
        Some(Command::Transport { command }) => execute_transport(command),
        Some(Command::Bootstrap { command }) => execute_bootstrap(command),
        Some(Command::Network { command }) => execute_network(command),
        Some(Command::Audit { command }) => execute_audit(command),
    }
}

fn registry_path(value: Option<String>) -> String {
    resolve_runtime_paths(&RuntimeContext::from_env(value))
        .registry_path
        .to_string_lossy()
        .to_string()
}

fn emit_json(value: &impl Serialize, should_fail: bool) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(payload) => {
            println!("{payload}");
            if should_fail {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => fail(error.to_string()),
    }
}

fn fail(error: String) -> ExitCode {
    eprintln!("{error}");
    ExitCode::FAILURE
}
