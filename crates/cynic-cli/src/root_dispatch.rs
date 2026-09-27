use std::process::ExitCode;

use crate::Command;
use crate::agent_surface::execute_agent;
use crate::build_surface::execute_build;
use crate::chat_surface::{ChatArgs, execute_chat};
use crate::cli_args::StatusCommand;
use crate::cli_json::emit_pretty_json;
use crate::command_manifest::commands_manifest;
use crate::deps_surface;
use crate::network_surface::execute_network;
use crate::node_activation_surface::execute_node_activation;
use crate::operator_doctor::execute_operator_doctor;
use crate::operator_errors::execute_operator_errors;
use crate::operator_help::execute_advanced_help;
use crate::runtime_surface::{execute_operator_models, execute_runtime};
use crate::session_surface::execute_session;

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
        Command::Session { command } => execute_session(&command),
        Command::Network { command } => execute_network(command),
        Command::Node { command } => execute_node_activation(command),
        Command::Advanced => execute_advanced_help(),
        Command::Runtime { command } => execute_runtime(command),
        Command::Update { target, json } => {
            let only = target.map(|t| vec![t]).unwrap_or_default();
            deps_surface::execute_deps_upgrade(json, false, true, None, &only)
        }
        Command::Status { command, json } => match command {
            None => crate::operator_status::execute_operator_status(json),
            Some(StatusCommand::Doctor { json: j }) => execute_operator_doctor(j || json),
            Some(StatusCommand::Models { command: c }) => execute_operator_models(c),
            Some(StatusCommand::Errors { json: j }) => execute_operator_errors(j || json),
            Some(StatusCommand::Version { json: j }) => {
                crate::version_surface::execute_version(j || json);
                ExitCode::SUCCESS
            }
        },
        Command::Audit { command } => crate::audit_surface::execute_audit(&command),
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
        Command::Court { command } => crate::constitutional_court_surface::execute_court(command),
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
        Command::Queue { json, apply } => crate::queue_surface::execute_queue(json, apply),
        Command::Forensics { command } => crate::forensics_surface::execute_forensics(command),
        Command::Discovery { command } => crate::discovery_surface::execute_discovery(command),
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

fn execute_chat_command(args: ChatArgs) -> ExitCode {
    execute_chat(&args)
}
