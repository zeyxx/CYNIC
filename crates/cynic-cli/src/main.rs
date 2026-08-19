#![allow(clippy::print_stdout, clippy::print_stderr)]
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
mod agent_exposure_audit;
use agent_surface::AgentCommand;
pub mod agent_surface;
mod audit_surface;
mod bootstrap_surface;
mod chat_surface;
mod chat_tui;
mod cli_exit;
mod cli_json;
mod command_manifest;
mod crate_audit;
mod deps_surface;
mod deps_surface_audit;
mod deps_surface_model;
mod deps_surface_pins;
mod deps_surface_plan;
mod deps_surface_probe;
mod deps_surface_render;
mod deps_surface_report;
mod deps_surface_specs;
mod deps_surface_upgrade;
mod dev_surface;
mod fleet_observation;
mod fleet_report;
mod fleet_surface;
mod fs_json;
mod host_exec_alias_surface;
mod integrations_readonly_surface;
mod integrations_surface;
mod integrations_surface_render;
mod kernel_boundary_audit;
mod lab_surface;
mod lifecycle_surface;
mod lifecycle_surface_context;
mod lifecycle_surface_execute;
mod lifecycle_surface_render;
mod network_surface;
mod node_host_audit;
pub(crate) mod federation_surface;
mod operator_doctor;
mod operator_errors;
mod operator_help;
mod operator_status;
mod ownership_audit;
mod root_dispatch;
mod runtime_paths;
mod runtime_suite;
mod runtime_surface;
mod rust_source;
mod solana_surface;

#[allow(dead_code)]
mod autoresearch_surface;
mod surface_audit;
mod surface_registry;
pub(crate) mod build_surface;
pub(crate) mod workload_observation;
mod transport_surface;
use audit_surface::AuditCommand;
use bootstrap_surface::BootstrapCommand;
use dev_surface::DevCommand;
use fleet_surface::FleetCommand;
use host_exec_alias_surface::maybe_run_host_exec_alias;
use integrations_surface::IntegrationsCommand;
use lab_surface::LabCommand;
use network_surface::NetworkCommand;
use root_dispatch::{execute_root_command, print_cli_help};
use runtime_paths::{RuntimeContext, resolve_runtime_paths};
use runtime_surface::{OperatorModelsCommand, RuntimeCommand};
use serde::Serialize;
use solana_surface::SolanaCommand;
use transport_surface::TransportCommand;

#[derive(Parser)]
#[command(
    name = "cynic",
    about = "Portable CYNIC operator CLI",
    after_help = "Start here: cynic status → cynic doctor → cynic models list → cynic chat\nAdvanced runtime surfaces: cynic advanced"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    #[command(hide = true)]
    Commands {
        #[arg(long)]
        json: bool,
    },
    #[command(hide = true)]
    Fleet {
        #[command(subcommand)]
        command: FleetCommand,
    },
    #[command(hide = true)]
    Runtime {
        #[command(subcommand)]
        command: RuntimeCommand,
    },
    /// Inspect and explicitly control local LLM model profiles.
    Models {
        #[command(subcommand)]
        command: OperatorModelsCommand,
    },
    /// Read-only diagnosis of local LLM, network, and sovereign Git readiness.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Explain stable CYNIC error codes and their operator remediation.
    Errors {
        #[arg(long)]
        json: bool,
    },
    /// Explain and list the advanced runtime surfaces.
    Advanced,
    /// Read-only Solana observation planning surfaces.
    #[command(hide = true)]
    Solana {
        #[command(subcommand)]
        command: SolanaCommand,
    },
    #[command(hide = true)]
    Dev {
        #[command(subcommand)]
        command: DevCommand,
    },
    #[command(hide = true)]
    Bootstrap {
        #[command(subcommand)]
        command: BootstrapCommand,
    },
    #[command(hide = true)]
    Lab {
        #[command(subcommand)]
        command: LabCommand,
    },
    #[command(hide = true)]
    Transport {
        #[command(subcommand)]
        command: TransportCommand,
    },
    #[command(hide = true)]
    Network {
        #[command(subcommand)]
        command: NetworkCommand,
    },
    #[command(hide = true)]
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
    /// Observe CYNIC-managed services and external provider connections.
    #[command(hide = true)]
    Integrations {
        #[command(subcommand)]
        command: IntegrationsCommand,
    },
    /// Ecosystem tech-watch and human-validated upgrades (canonical).
    #[command(hide = true)]
    Deps {
        #[command(subcommand)]
        command: Option<DepsCommand>,
        /// Machine-readable JSON (default is human text).
        #[arg(long, global = true)]
        json: bool,
    },
    /// Update lifecycle: lab → conformance → staging → canary → production.
    #[command(hide = true)]
    Lifecycle {
        #[command(subcommand)]
        command: lifecycle_surface::LifecycleCommand,
    },
    /// One-word frictionless update for ecosystem components (hermes-agent, llama.cpp, cynic, etc.).
    Update {
        /// Optional target component id (e.g. hermes-agent). Omit to upgrade all stale components.
        target: Option<String>,
        /// Machine-readable JSON report.
        #[arg(long)]
        json: bool,
    },
    /// Read-only status of declared inference state and observed local LLM supervision.
    Status {
        /// Machine-readable JSON report.
        #[arg(long)]
        json: bool,
    },
    /// HumanAdmin governance observer surfaces.
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
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
        /// Force model selector (local alias when present, otherwise model id).
        #[arg(long)]
        model: Option<String>,
        /// Do not open the interactive TUI when prompt is omitted.
        #[arg(long = "no-tui")]
        no_tui: bool,
    },
    /// Asynchronous domain-partitioned task queue (Inference, Infra, CodeAudit, Cortex, Governance, Nightshift).
    Task {
        #[command(subcommand)]
        command: task_surface::TaskCommand,
    },
    /// Federated cluster status, Synapse WireGuard mesh probes, and cross-node sync.
    Federation {
        #[command(subcommand)]
        command: federation_surface::FederationCommand,
    },
    /// Trigger a distributed build on the network runner for an exact commit.
    Build {
        #[command(subcommand)]
        command: build_surface::BuildCommand,
    },
}

#[derive(Subcommand)]
enum DepsCommand {
    /// Ordered upgrade plan (manual + one-click items).
    Plan,
    /// Prioritized normalization audit for Python/system CLI authorities.
    Audit,
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
            print_cli_help(&mut command)
        }
        Some(command) => execute_root_command(command),
    }
}

fn registry_path(value: Option<String>) -> String {
    resolve_runtime_paths(&RuntimeContext::from_env(value))
        .registry_path
        .to_string_lossy()
        .to_string()
}

pub(crate) fn emit_json(value: &impl Serialize, should_fail: bool) -> ExitCode {
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

pub(crate) fn fail(error: impl AsRef<str>) -> ExitCode {
    eprintln!("{}", error.as_ref());
    ExitCode::FAILURE
}
