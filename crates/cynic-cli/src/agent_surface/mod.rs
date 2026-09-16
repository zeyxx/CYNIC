//! Agent coordination CLI surface — thin clap facade over kernel REST.
//!
//! Commands:
//!   - cynic agent identify  — declare agent identity
//!   - cynic agent register   — register with kernel
//!   - cynic agent claim      — claim a scope for exclusive editing
//!   - cynic agent release    — release an active claim
//!   - cynic agent status     — query active claims
//!   - cynic agent intent seal|verify — seal or verify transmission records

use std::process::ExitCode;

use clap::Subcommand;

use crate::cli_json::emit_pretty_json;

use self::coordination::{
    execute_claim, execute_identify, execute_register, execute_release, execute_status,
    resolve_kernel_base,
};
use self::intent::{IntentCommand, execute_intent};

pub mod coordination;
pub mod intent;
pub mod render;
pub mod scan;
pub mod types;

#[derive(Subcommand)]
pub(crate) enum AgentCommand {
    /// Declare agent identity with attested credentials
    Identify {
        /// Agent ID (unique within organism)
        #[arg(long, env = "CYNIC_AGENT_ID")]
        agent_id: String,
        /// Agent kind (hermes, codex, claude, cynic, human_admin)
        #[arg(long)]
        agent_kind: Option<String>,
        /// Registry path used to resolve the kernel REST base.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Kernel REST base override (else CYNIC_KERNEL_BASE env, else registry).
        #[arg(long)]
        kernel_base: Option<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Register this agent with the kernel coordination system
    Register {
        /// Agent ID
        #[arg(long, env = "CYNIC_AGENT_ID")]
        agent_id: String,
        /// Registration intent (objective statement)
        #[arg(long, default_value = "sovereign-agent-session")]
        intent: String,
        /// Registry path used to resolve the kernel REST base.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Kernel REST base override (else CYNIC_KERNEL_BASE env, else registry).
        #[arg(long)]
        kernel_base: Option<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Claim a scope for exclusive editing
    Claim {
        /// Agent ID
        #[arg(long, env = "CYNIC_AGENT_ID")]
        agent_id: String,
        /// Target path or resource to claim
        #[arg(long)]
        target: String,
        /// Claim type (file, crate, module, exclusive, shared_observe)
        #[arg(long, default_value = "file")]
        claim_type: Option<String>,
        /// TTL in seconds (30-86400)
        #[arg(long, default_value = "1800")]
        ttl_seconds: Option<u64>,
        /// Action budget (max mutations)
        #[arg(long, default_value = "50")]
        budget_actions: Option<u64>,
        /// Wall-time budget (seconds)
        #[arg(long, default_value = "3600")]
        budget_wall_seconds: Option<u64>,
        /// Registry path used to resolve the kernel REST base.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Kernel REST base override (else CYNIC_KERNEL_BASE env, else registry).
        #[arg(long)]
        kernel_base: Option<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Release a previously claimed scope
    Release {
        /// Agent ID
        #[arg(long, env = "CYNIC_AGENT_ID")]
        agent_id: String,
        /// Target that was claimed
        #[arg(long)]
        target: String,
        /// Lease ID from claim response
        #[arg(long)]
        lease_id: String,
        /// Fencing token from claim response (kernel authority; ignored by
        /// the local claims fallback, defaults to 0)
        #[arg(long, default_value = "0")]
        fencing_token: u64,
        /// Proof references (evidence of completed work)
        #[arg(long)]
        proof_ref: Vec<String>,
        /// Registry path used to resolve the kernel REST base.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Kernel REST base override (else CYNIC_KERNEL_BASE env, else registry).
        #[arg(long)]
        kernel_base: Option<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Query active claims status
    Status {
        /// Agent ID (defaults to all active claims when omitted)
        #[arg(long, env = "CYNIC_AGENT_ID", default_value = "*")]
        agent_id: String,
        /// Registry path used to resolve the kernel REST base.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Kernel REST base override (else CYNIC_KERNEL_BASE env, else registry).
        #[arg(long)]
        kernel_base: Option<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Return lightweight non-blocking session context evidence for agent boots
    SessionContext {
        /// Print JSON output (default true)
        #[arg(long, default_value = "true")]
        json: bool,
    },
    /// Seal or verify intent transmission records (nested arm, no new top level)
    Intent {
        #[command(subcommand)]
        command: IntentCommand,
    },
    /// Plan or observe Proxmox agent host VM placement and conformance.
    Host {
        #[command(subcommand)]
        command: crate::agent_host_surface::AgentHostCommand,
    },
}

pub(crate) fn execute_agent(command: AgentCommand) -> ExitCode {
    match command {
        AgentCommand::Identify {
            agent_id,
            agent_kind,
            registry,
            kernel_base,
            json: _,
        } => match execute_identify(
            &agent_id,
            agent_kind.as_deref(),
            resolve_kernel_base(&registry, kernel_base.as_deref()).as_deref(),
        ) {
            Ok(report) => {
                emit_pretty_json(&report);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("agent identify failed: {}", error);
                ExitCode::FAILURE
            }
        },
        AgentCommand::Register {
            agent_id,
            intent,
            registry,
            kernel_base,
            json: _,
        } => match execute_register(
            &agent_id,
            &intent,
            resolve_kernel_base(&registry, kernel_base.as_deref()).as_deref(),
        ) {
            Ok(()) => {
                let report = serde_json::json!({
                    "schema": "cynic.cli.agent_register_report.v1",
                    "status": "registered",
                    "agent_id": agent_id,
                });
                emit_pretty_json(&report);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("agent register failed: {}", error);
                ExitCode::FAILURE
            }
        },
        AgentCommand::Claim {
            agent_id,
            target,
            claim_type,
            ttl_seconds,
            budget_actions,
            budget_wall_seconds,
            registry,
            kernel_base,
            json: _,
        } => match execute_claim(
            &agent_id,
            &target,
            claim_type.as_deref(),
            ttl_seconds,
            budget_actions,
            budget_wall_seconds,
            resolve_kernel_base(&registry, kernel_base.as_deref()).as_deref(),
        ) {
            Ok(report) => {
                emit_pretty_json(&report);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("agent claim failed: {}", error);
                ExitCode::FAILURE
            }
        },
        AgentCommand::Release {
            agent_id,
            target,
            lease_id,
            fencing_token,
            proof_ref,
            registry,
            kernel_base,
            json: _,
        } => match execute_release(
            &agent_id,
            &target,
            &lease_id,
            fencing_token,
            proof_ref,
            resolve_kernel_base(&registry, kernel_base.as_deref()).as_deref(),
        ) {
            Ok(report) => {
                emit_pretty_json(&report);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("agent release failed: {}", error);
                ExitCode::FAILURE
            }
        },
        AgentCommand::Status {
            agent_id,
            registry,
            kernel_base,
            json: _,
        } => match execute_status(
            &agent_id,
            resolve_kernel_base(&registry, kernel_base.as_deref()).as_deref(),
        ) {
            Ok(report) => {
                emit_pretty_json(&report);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("agent status failed: {}", error);
                ExitCode::FAILURE
            }
        },
        AgentCommand::SessionContext { json: _ } => {
            let report = serde_json::json!({
                "schema": "cynic.agent.session_context.v1",
                "status": "observed",
                "git_sha": env!("CYNIC_GIT_SHA"),
                "version": env!("CARGO_PKG_VERSION"),
                "canonical_service_plane": "synapse",
                "builder": "10.10.10.119",
                "git_remote": "ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git",
                "active_claims_file": ".agent-claims.json",
            });
            emit_pretty_json(&report);
            ExitCode::SUCCESS
        }
        AgentCommand::Intent { command } => execute_intent(command),
        AgentCommand::Host { command } => crate::agent_host_surface::execute_agent_host(command),
    }
}
