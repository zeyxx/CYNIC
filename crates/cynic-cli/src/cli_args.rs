use clap::{Parser, Subcommand};

use crate::agent_surface::AgentCommand;
use crate::audit_surface::AuditCommand;
use crate::build_surface::BuildCommand;
use crate::federation_surface::FederationCommand;
use crate::forensics_surface::ForensicsCommand;
use crate::git_surface::GitCommand;
use crate::judge_surface::JudgeCommand;
use crate::mask_surface::MaskCommand;
use crate::network_surface::NetworkCommand;
use crate::node_activation_surface::NodeActivationCommand;
use crate::runtime_surface::OperatorModelsCommand;
use crate::session_surface::SessionCommand;
use crate::source_surface::SourceCommand;
use crate::task_surface::TaskCommand;

#[derive(Parser)]
#[command(
    name = "cynic",
    version = env!("CARGO_PKG_VERSION"),
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
    /// Constitutional court — audit policy alignment across surfaces.
    Court {
        #[command(subcommand)]
        command: crate::constitutional_court_surface::CourtCommand,
    },
    /// Discovery — proactive capability declaration and fleet observation.
    Discovery {
        #[command(subcommand)]
        command: crate::discovery_surface::DiscoveryCommand,
    },
    /// Explain stable CYNIC error codes and their operator remediation.
    Errors {
        #[arg(long)]
        json: bool,
    },
    /// Explain and list the advanced runtime surfaces.
    Advanced,
    /// Read-only tmux session supervision with per-pane metrics and activity.
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    /// Unified network management, observation, mesh planning, peer consensus, and registry audit.
    Network {
        #[command(subcommand)]
        command: NetworkCommand,
    },
    /// Observe and manage node identity, enrollment certificates, and lifecycle activation.
    Node {
        #[command(subcommand)]
        command: NodeActivationCommand,
    },
    /// Source sync — observe/reconcile/apply across layers + release bot
    Source {
        #[command(subcommand)]
        command: SourceCommand,
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
        #[command(subcommand)]
        command: Option<StatusCommand>,
        /// Machine-readable JSON report.
        #[arg(long)]
        json: bool,
    },
    /// Dynamic architectural invariants, secret scan & contract verification.
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
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
        command: TaskCommand,
    },
    /// Federated cluster status, Synapse WireGuard mesh probes, and cross-node sync.
    Federation {
        #[command(subcommand)]
        command: FederationCommand,
    },
    /// Execute, submit, and observe reproducible sovereign builds.
    Build {
        #[command(subcommand)]
        command: BuildCommand,
    },
    /// Launch an agent harness through a cynic mask (forge profile → start).
    Mask {
        #[command(subcommand)]
        command: MaskCommand,
    },
    /// Local queue plan — branch+discovery+GC composed via LocalQueueAdapter (thin).
    Queue {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Forensics — read-only projection of host crash evidence.
    Forensics {
        #[command(subcommand)]
        command: ForensicsCommand,
    },
    /// Binary source identity: exact git SHA baked at compile time.
    Version {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum StatusCommand {
    /// Read-only diagnosis of local LLM, network, and sovereign Git readiness.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Inspect and explicitly control local LLM model profiles.
    Models {
        #[command(subcommand)]
        command: OperatorModelsCommand,
    },
    /// Explain stable CYNIC error codes and their operator remediation.
    Errors {
        #[arg(long)]
        json: bool,
    },
    /// Binary source identity: exact git SHA baked at compile time.
    Version {
        #[arg(long)]
        json: bool,
    },
}
