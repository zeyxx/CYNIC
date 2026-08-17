//! Governance observation surface ("SOC-lite") for HumanAdmin visibility.
//!
//! `cynic agent soc --json` projects the RBAC read-back state from
//! `infra/agent-roles.json` reconciled against observed runtime proof.
//! This surface NEVER mutates — it satisfies the HumanAdmin desire for
//! maximal visibility without touching runtime execution.
//!
//! Audit grammar: facade -> types -> scan -> render.
//! Read-back proof surface: MachineCommandExitCode::Success only when the
//! projection is internally consistent (HumanAdmin zero-touch invariant holds).

use std::path::Path;

use clap::Subcommand;

use crate::cli_json::emit_pretty_json;

mod render;
pub mod scan;
pub mod types;

#[allow(unused_imports)]
pub(crate) use types::{AgentSocEntry, AgentSocReport, DomainStatus, HumanAdminStatus};

pub(crate) use render::render_text;

pub const DEFAULT_ROLES_FILE: &str = "infra/agent-roles.json";

#[derive(Subcommand)]
pub(crate) enum AgentCommand {
    /// Read-back of declared agent roles vs observed runtime proof (HumanAdmin visibility).
    Soc {
        /// Machine-readable JSON report.
        #[arg(long)]
        json: bool,
        /// Path to infra/agent-roles.json (default: repo-root infra/).
        #[arg(long, value_name = "PATH")]
        roles_file: Option<String>,
    },
}

#[allow(dead_code)]
pub(crate) fn execute_agent(command: AgentCommand) -> std::process::ExitCode {
    match command {
        AgentCommand::Soc {
            json,
            roles_file,
        } => {
            let path = roles_file.as_deref().unwrap_or(DEFAULT_ROLES_FILE);
            let report = scan::scan_agent_soc(Path::new(path));
            if json {
                let _ = emit_pretty_json(&report);
            } else {
                render_text(&report);
            }
            // Exit 0 only if HumanAdmin zero-touch invariant holds.
            if report.human_admin_role.zero_touch_runtime {
                std::process::ExitCode::SUCCESS
            } else {
                std::process::ExitCode::FAILURE
            }
        }
    }
}
