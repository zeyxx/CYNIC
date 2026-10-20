//! Agent session family — nested arm, no hyphenated command name.
//!
//! WHY: `cynic agent session-context` carried a hyphen, which
//! contracts/foundation.json forbids for every CLI command. The rule is
//! "nest as an arm of an owning family", so the command became
//! `cynic agent session context`: a `Session` arm holding a
//! `SessionContext` subcommand, the same shape as `agent intent` and as
//! the top-level `session` family.
//!
//! HOW: Parse argv, resolve the builder endpoint from the registry, print
//! the session context evidence as JSON. Exit code is the machine contract:
//! 0 on success, 1 on failure.
//! NEGATIVE BOUNDARY: Read-only. It observes the runtime and the registry;
//! it never mutates a claim, a session or a registry entry.

use std::process::ExitCode;

use clap::Subcommand;

use crate::agent_surface::coordination::{resolve_builder_endpoint, resolve_git_remote};
use crate::cli_json::emit_pretty_json;

/// `cynic agent session ...` — nested arm, zero new top-level hyphens.
#[derive(Subcommand)]
pub(crate) enum AgentSessionCommand {
    /// Return lightweight non-blocking session context evidence for agent boots
    Context {
        /// Print JSON output (default true)
        #[arg(long, default_value = "true")]
        json: bool,
        /// Registry path used to resolve the builder endpoint.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
    },
}

pub(crate) fn execute_agent_session(command: &AgentSessionCommand) -> ExitCode {
    match command {
        AgentSessionCommand::Context { json: _, registry } => {
            let report = serde_json::json!({
                "schema": "cynic.agent.session_context.v1",
                "status": "observed",
                "git_sha": env!("CYNIC_GIT_SHA"),
                "version": env!("CARGO_PKG_VERSION"),
                "canonical_service_plane": "service",
                "builder": resolve_builder_endpoint(&registry).unwrap_or("unknown".to_string()),
                "git_remote": resolve_git_remote(),
                "active_claims_file": ".agent-claims.json",
            });
            emit_pretty_json(&report);
            ExitCode::SUCCESS
        }
    }
}
