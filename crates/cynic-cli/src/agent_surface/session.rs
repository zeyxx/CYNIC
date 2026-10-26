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
use cynic_knowledge;

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
            let repo_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let repo_root = find_repo_root(&repo_root).unwrap_or(repo_root);
            let index = cynic_knowledge::index_workspace(&repo_root);

            // Knowledge hints: top chunks for key orientation queries
            let mut hints = Vec::new();
            for query in ["session compass", "engineering patterns"] {
                for hit in index.search(query, 2) {
                    hints.push(serde_json::json!({
                        "query": query,
                        "source": hit.chunk.source_path,
                        "section": hit.chunk.title(),
                        "preview": hit.chunk.content.lines().take(3).collect::<Vec<_>>().join(" "),
                    }));
                }
            }

            let report = serde_json::json!({
                "schema": "cynic.agent.session_context.v1",
                "status": "observed",
                "git_sha": env!("CYNIC_GIT_SHA"),
                "version": env!("CARGO_PKG_VERSION"),
                "canonical_service_plane": "synapse",
                "builder": resolve_builder_endpoint(&registry).unwrap_or("unknown".to_string()),
                "git_remote": resolve_git_remote(),
                "active_claims_file": ".agent-claims.json",
                "knowledge_hints": hints,
            });
            emit_pretty_json(&report);
            ExitCode::SUCCESS
        }
    }
}

fn find_repo_root(start: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut current = start;
    loop {
        if current.join("Cargo.toml").exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}
