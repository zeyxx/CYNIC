use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
mod agent_exposure_audit;
mod audit_surface;
mod bootstrap_surface;
mod chat_surface;
mod chat_tui;
mod command_manifest;
mod crate_audit;
mod deps_surface;
mod dev_surface;
mod fleet_observation;
mod fleet_report;
mod fleet_surface;
mod host_exec_alias_surface;
mod kernel_boundary_audit;
mod lab_surface;
mod lifecycle_surface;
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
    /// Ecosystem tech-watch and human-validated upgrades (canonical).
    Deps {
        #[command(subcommand)]
        command: Option<DepsCommand>,
        /// Machine-readable JSON (default is human text).
        #[arg(long, global = true)]
        json: bool,
    },
    /// Update lifecycle: lab → conformance → staging → canary → production.
    Lifecycle {
        #[command(subcommand)]
        command: lifecycle_surface::LifecycleCommand,
    },
    /// Operator chat against the inference plane (session history under principal).
    /// Default with no prompt: interactive TUI. Does not require cynic-kernel.
    Chat {
        /// User prompt (one-shot). Omit to open the interactive TUI.
        prompt: Option<String>,
        /// Resume an existing session id (one-shot).
        #[arg(long)]
        resume: Option<String>,
        /// Explicit session id for a new or continuing session.
        #[arg(long = "session")]
        session_id: Option<String>,
        /// Request progressive stream chunks in the turn report / TUI turns.
        #[arg(long)]
        stream: bool,
        /// Machine-readable JSON (disables TUI).
        #[arg(long)]
        json: bool,
        /// List durable sessions under principal chat state.
        #[arg(long = "list-sessions")]
        list_sessions: bool,
        /// Export a session as JSONL to stdout.
        #[arg(long)]
        export: Option<String>,
        /// Force mock inference plane (tools/stream without live engine).
        #[arg(long)]
        mock: bool,
        /// Prefer Osmantic ODS OpenAI-compat discovery for the chat endpoint.
        #[arg(long)]
        ods: bool,
        /// Do not open the interactive TUI when prompt is omitted.
        #[arg(long = "no-tui")]
        no_tui: bool,
    },
}

#[derive(Subcommand)]
enum DepsCommand {
    /// Ordered upgrade plan (manual + one-click items).
    Plan,
    /// Preview or apply scripted upgrades (`--yes` = one-click human consent).
    Upgrade {
        /// Apply all allowlisted scripted upgrades (explicit human consent).
        #[arg(long)]
        yes: bool,
        /// Non-interactive apply (agents): requires --confirm from a plan.
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        confirm: Option<String>,
        /// Limit to dep ids (e.g. cynic, huggingface-cli).
        #[arg(long = "only")]
        only: Vec<String>,
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
        Some(Command::Deps { command, json }) => match command {
            None => deps_surface::execute_deps(json),
            Some(DepsCommand::Plan) => deps_surface::execute_deps_plan(json),
            Some(DepsCommand::Upgrade {
                yes,
                apply,
                confirm,
                only,
            }) => deps_surface::execute_deps_upgrade(json, apply, yes, confirm.as_deref(), &only),
        },
        Some(Command::Lifecycle { command }) => lifecycle_surface::execute_lifecycle(command),
        Some(Command::Chat {
            prompt,
            resume,
            session_id,
            stream,
            json,
            list_sessions,
            export,
            mock,
            ods,
            no_tui,
        }) => chat_surface::execute_chat(chat_surface::ChatArgs {
            prompt,
            resume,
            session_id,
            stream,
            json,
            list_sessions,
            export,
            mock,
            ods,
            no_tui,
        }),
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
