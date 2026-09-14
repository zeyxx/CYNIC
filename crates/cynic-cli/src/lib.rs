#![allow(clippy::print_stdout, clippy::print_stderr, dead_code)]
//! `cynic-cli` — Portable CYNIC operator CLI library.
//!
//! Conforms to the Library-First Pattern (§472): all domain code, command
//! structures, and dispatch logic live here; `main.rs` is a thin entrypoint.

use clap::{CommandFactory, Parser, Subcommand};
use serde::Serialize;
use std::process::ExitCode;

pub mod agent_exposure_audit;
pub mod agent_host_surface;
pub mod agent_surface;
pub mod audit_surface;
pub mod autoresearch_surface;
pub mod bootstrap_surface;
pub mod branch_graph_surface;
pub mod build_discovery_surface;
pub mod build_proof_log;
pub mod build_surface;
pub mod chat_surface;
pub mod chat_tui;
pub mod cli_exit;
pub mod cli_json;
pub mod command_manifest;
pub mod crate_audit;
pub mod debt_audit;
pub mod deps_surface;
pub mod deps_surface_audit;
pub mod deps_surface_model;
pub mod deps_surface_pins;
pub mod deps_surface_plan;
pub mod deps_surface_probe;
pub mod deps_surface_render;
pub mod deps_surface_report;
pub mod deps_surface_specs;
pub mod deps_surface_upgrade;
pub mod dev_surface;
pub mod doctor;
pub mod federation_surface;
pub mod fleet_divergence;
pub mod fleet_observation;
pub mod fleet_report;
pub mod fleet_surface;
pub mod forensics_surface;
pub mod fs_json;
pub mod git_surface;
pub mod host_exec_alias_surface;
pub mod integrations_readonly_surface;
pub mod integrations_surface;
pub mod integrations_surface_render;
pub mod invoice_log;
pub mod judge_surface;
pub mod judgment_surface;
pub mod kernel_boundary_audit;
pub mod lab_surface;
pub mod lifecycle_surface;
pub mod lifecycle_surface_context;
pub mod lifecycle_surface_execute;
pub mod lifecycle_surface_render;
pub mod mask_surface;
pub mod mcp_call_proof_index;

pub mod network_audit;
pub mod network_surface;
pub mod node_activation_surface;
pub mod node_host_audit;
pub mod operator_doctor;
pub mod operator_errors;
pub mod operator_help;
pub mod operator_status;
pub mod ownership_audit;
pub mod peer_surface;
pub mod policy_formality_audit;
pub mod queue_surface;
pub mod root_dispatch;
pub mod runtime_paths;
pub mod runtime_suite;
pub mod runtime_surface;
pub mod rust_source;
pub mod schema_registry_audit;
pub mod session_surface;
pub mod solana_surface;
pub mod source_surface;
pub mod surface_audit;
pub mod surface_registry;
pub mod task_surface;
pub mod transport_surface;
pub mod version_surface;
pub mod workload_observation;

use crate::agent_surface::AgentCommand;
use crate::audit_surface::AuditCommand;
use crate::bootstrap_surface::BootstrapCommand;
use crate::dev_surface::DevCommand;
use crate::fleet_surface::FleetCommand;
use crate::git_surface::GitCommand;
use crate::host_exec_alias_surface::maybe_run_host_exec_alias;
use crate::integrations_surface::IntegrationsCommand;
use crate::judge_surface::JudgeCommand;
use crate::lab_surface::LabCommand;
use crate::network_audit::NetworkAuditCommand;
use crate::network_surface::NetworkCommand;
use crate::node_activation_surface::NodeActivationCommand;
use crate::root_dispatch::{execute_root_command, print_cli_help};
use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};
use crate::runtime_surface::{OperatorModelsCommand, RuntimeCommand};
use crate::solana_surface::SolanaCommand;
use crate::transport_surface::TransportCommand;

#[derive(Parser)]
#[command(
    name = "cynic",
    version = concat!(env!("CARGO_PKG_VERSION"), " (release 2026.09.12)"),
    about = "Portable CYNIC operator CLI",
    after_help = "Start here: cynic status → cynic doctor → cynic models list → cynic chat\nAdvanced runtime surfaces: cynic advanced"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Subcommand)]
pub(crate) enum Command {
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
    /// Read-only network audit: declared (registry) vs observed (DNS, WireGuard, services).
    NetworkAudit {
        #[command(subcommand)]
        command: NetworkAuditCommand,
    },
    /// Read-only tmux session supervision with per-pane metrics and activity.
    Session {
        #[command(subcommand)]
        command: session_surface::SessionCommand,
    },
    #[command(hide = true)]
    Network {
        #[command(subcommand)]
        command: NetworkCommand,
    },
    /// CynicPeer sovereign state engine, local storage and ingress commands.
    Peer {
        #[command(subcommand)]
        command: cynic_node::cli::PeerCommands,
    },
    /// Observe and manage node activation (deployment_epoch + bootstrap + takeover).
    NodeActivation {
        #[command(subcommand)]
        command: NodeActivationCommand,
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
    /// Source sync — observe/reconcile/apply across layers + release bot
    Source {
        #[command(subcommand)]
        command: source_surface::SourceCommand,
    },
    /// Git branch topology — observe/reconcile native supervision.
    Git {
        #[command(subcommand)]
        command: GitCommand,
    },
    /// Judge extracted patches (diff vs origin/main) — data-centric gate before adding.
    Judge {
        #[command(subcommand)]
        command: JudgeCommand,
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
    /// Execute, submit, and observe reproducible sovereign builds.
    Build {
        #[command(subcommand)]
        command: build_surface::BuildCommand,
    },
    /// Launch an agent harness through a cynic mask (forge profile → start).
    Mask {
        #[command(subcommand)]
        command: mask_surface::MaskCommand,
    },
    /// Judgment projection and canonical content-addressed digest.
    #[command(hide = true)]
    Judgment {
        #[command(subcommand)]
        command: judgment_surface::JudgmentCommand,
    },
    /// Forensics — post-crash evidence, timeline, post-mortem.
    #[command(hide = true)]
    Forensics {
        #[command(subcommand)]
        command: forensics_surface::ForensicsCommand,
    },
    /// Agent host VM placement and conformance planning.
    #[command(name = "agent-host")]
    AgentHost {
        #[command(subcommand)]
        command: agent_host_surface::AgentHostCommand,
    },
    /// Local queue plan — branch+discovery+GC composed via LocalQueueAdapter (thin).
    Queue {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Branch graph — thin projection via LocalBranchGraphAdapter.
    Branch {
        #[command(subcommand)]
        command: BranchCommand,
    },
    /// Binary source identity: exact git SHA baked at compile time.
    Version {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum BranchCommand {
    /// Branch graph parent inference (BranchGraph {parent Option}).
    Graph {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum DepsCommand {
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

pub fn run_main() -> ExitCode {
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

pub fn registry_path(value: Option<String>) -> String {
    resolve_runtime_paths(&RuntimeContext::from_env(value))
        .registry_path
        .to_string_lossy()
        .to_string()
}

pub fn emit_json(value: &impl Serialize, should_fail: bool) -> ExitCode {
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

pub fn fail(error: impl AsRef<str>) -> ExitCode {
    eprintln!("{}", error.as_ref());
    ExitCode::FAILURE
}
