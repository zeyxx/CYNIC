use std::process::ExitCode;

use crate::audit_surface::{AuditCommand, execute_audit};
use crate::agent_surface::execute_agent;
use crate::bootstrap_surface::{BootstrapCommand, execute_bootstrap};
use crate::chat_surface::{ChatArgs, execute_chat};
use crate::cli_json::emit_pretty_json;
use crate::command_manifest::commands_manifest;
use crate::deps_surface;
use crate::dev_surface::{DevCommand, execute_dev};
use crate::fleet_surface::{FleetCommand, execute_fleet};
use crate::integrations_surface::{IntegrationsCommand, execute_integrations};
use crate::lab_surface::{LabCommand, execute_lab};
use crate::network_surface::{NetworkCommand, execute_network};
use crate::runtime_surface::{RuntimeCommand, execute_runtime};
use crate::solana_surface::{SolanaCommand, execute_solana};
use crate::transport_surface::{TransportCommand, execute_transport};
use crate::{Command, DepsCommand};

pub(crate) fn print_cli_help(command: &mut clap::Command) -> ExitCode {
    match command.print_help() {
        Ok(()) => {
            println!();
            ExitCode::SUCCESS
        }
        Err(_) => ExitCode::FAILURE,
    }
}

pub(crate) fn execute_root_command(command: Command) -> ExitCode {
    match command {
        Command::Commands { json } => execute_commands_manifest(json),
        Command::Fleet { command } => execute_fleet_command(command),
        Command::Runtime { command } => execute_runtime_command(command),
        Command::Solana { command } => execute_solana_command(command),
        Command::Dev { command } => execute_dev_command(command),
        Command::Bootstrap { command } => execute_bootstrap_command(command),
        Command::Lab { command } => execute_lab_command(command),
        Command::Transport { command } => execute_transport_command(command),
        Command::Network { command } => execute_network_command(command),
        Command::Audit { command } => execute_audit_command(command),
        Command::Integrations { command } => execute_integrations_command(command),
        Command::Deps { command, json } => execute_deps_command(command, json),
        Command::Update { target, json } => {
            let only = target.map(|t| vec![t]).unwrap_or_default();
            deps_surface::execute_deps_upgrade(json, false, true, None, &only)
        }
        Command::Status { json } => deps_surface::execute_deps(json),
        Command::Lifecycle { command } => crate::lifecycle_surface::execute_lifecycle(command),
        Command::Chat {
            prompt,
            resume,
            session_id,
            stream,
            json,
            list_sessions,
            export,
            mock,
            ods,
            model,
            no_tui,
        } => execute_chat_command(ChatArgs {
            prompt,
            resume,
            session_id,
            stream,
            json,
            list_sessions,
            export,
            mock,
            ods,
            model,
            no_tui,
        }),
        Command::Agent { command } => execute_agent(command),
    }
}

fn execute_commands_manifest(json: bool) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    emit_pretty_json(&commands_manifest())
}

fn execute_fleet_command(command: FleetCommand) -> ExitCode {
    execute_fleet(command)
}

fn execute_runtime_command(command: RuntimeCommand) -> ExitCode {
    execute_runtime(command)
}

fn execute_solana_command(command: SolanaCommand) -> ExitCode {
    execute_solana(command)
}

fn execute_dev_command(command: DevCommand) -> ExitCode {
    execute_dev(command)
}

fn execute_bootstrap_command(command: BootstrapCommand) -> ExitCode {
    execute_bootstrap(command)
}

fn execute_lab_command(command: LabCommand) -> ExitCode {
    execute_lab(command)
}

fn execute_transport_command(command: TransportCommand) -> ExitCode {
    execute_transport(&command)
}

fn execute_network_command(command: NetworkCommand) -> ExitCode {
    execute_network(command)
}

fn execute_audit_command(command: AuditCommand) -> ExitCode {
    execute_audit(&command)
}

fn execute_integrations_command(command: IntegrationsCommand) -> ExitCode {
    execute_integrations(command)
}

fn execute_deps_command(command: Option<DepsCommand>, json: bool) -> ExitCode {
    match command {
        None => deps_surface::execute_deps(json),
        Some(DepsCommand::Plan) => deps_surface::execute_deps_plan(json),
        Some(DepsCommand::Audit) => deps_surface::execute_deps_audit(json),
        Some(DepsCommand::Upgrade {
            yes,
            apply,
            confirm,
            only,
        }) => deps_surface::execute_deps_upgrade(json, apply, yes, confirm.as_deref(), &only),
    }
}

fn execute_chat_command(args: ChatArgs) -> ExitCode {
    execute_chat(&args)
}
