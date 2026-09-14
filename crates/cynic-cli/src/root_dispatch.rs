use std::process::ExitCode;

use crate::agent_surface::execute_agent;
use crate::audit_surface::{AuditCommand, execute_audit};
use crate::bootstrap_surface::{BootstrapCommand, execute_bootstrap};
use crate::build_surface::execute_build;
use crate::chat_surface::{ChatArgs, execute_chat};
use crate::cli_json::emit_pretty_json;
use crate::command_manifest::commands_manifest;
use crate::deps_surface;
use crate::dev_surface::{DevCommand, execute_dev};
use crate::fleet_surface::{FleetCommand, execute_fleet};
use crate::forensics_surface::execute_forensics;
use crate::network_surface::{NetworkCommand, execute_network};
use crate::node_activation_surface::execute_node_activation;
use crate::operator_doctor::execute_operator_doctor;
use crate::operator_errors::execute_operator_errors;
use crate::operator_help::execute_advanced_help;
use crate::runtime_surface::{RuntimeCommand, execute_operator_models, execute_runtime};
use crate::session_surface::execute_session;
use crate::solana_surface::{SolanaCommand, execute_solana};
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
        Command::Dev { command } => execute_dev(command),
        Command::Bootstrap { command } => execute_bootstrap(command),
        Command::Session { command } => execute_session(&command),
        Command::Network { command } => execute_network(command),
        Command::Node { command } => execute_node_activation(command),
        Command::Audit { command } => execute_audit_command(command),
        Command::Advanced => execute_advanced_help(),
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
        Command::Models { command } => execute_operator_models(command),
        Command::Doctor { json } => execute_operator_doctor(json),
        Command::Errors { json } => execute_operator_errors(json),
        Command::Task { command } => crate::task_surface::execute_task(command),
        Command::Federation { command } => crate::federation_surface::execute_federation(command),
        Command::Build { command } => execute_build(command)
            .map(|_| ExitCode::SUCCESS)
            .unwrap_or_else(|e| {
                eprintln!("build error: {e}");
                ExitCode::FAILURE
            }),
        Command::Source { command } => crate::source_surface::execute_source(command),
        Command::Git { command } => crate::git_surface::execute_git(command),
        Command::Judge { command } => crate::judge_surface::execute_judge(command),
        Command::Mask { command } => crate::mask_surface::execute_mask(command),
        Command::Forensics { command } => execute_forensics(command),
        Command::Queue { json, apply } => crate::queue_surface::execute_queue(json, apply),
        Command::Version { json } => {
            crate::version_surface::execute_version(json);
            ExitCode::SUCCESS
        }
    }
}

fn execute_commands_manifest(json: bool) -> ExitCode {
    let manifest = commands_manifest();
    if json {
        return emit_pretty_json(&manifest);
    }
    println!("CYNIC command manifest ({})", manifest.schema);
    for domain in &manifest.domains {
        println!();
        println!("[{}]", domain.name);
        for command in &domain.commands {
            println!("  {command}");
        }
    }
    println!();
    println!("Machine-readable: cynic commands --json");
    ExitCode::SUCCESS
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

fn execute_network_command(command: NetworkCommand) -> ExitCode {
    execute_network(command)
}

fn execute_audit_command(command: AuditCommand) -> ExitCode {
    execute_audit(&command)
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
