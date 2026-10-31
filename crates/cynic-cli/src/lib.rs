#![allow(clippy::print_stdout, clippy::print_stderr)]
//! `cynic-cli` — Portable CYNIC operator CLI library.
//!
//! Conforms to the Library-First Pattern (§472): all domain code, command
//! structures, and dispatch logic live here; `main.rs` is a thin entrypoint.

use clap::{CommandFactory, Parser};
use serde::Serialize;
use std::process::ExitCode;

pub mod agent_host_surface;
pub mod agent_surface;
pub mod audit_surface;
pub mod collaborator_surface;
pub mod fleet_surface;
pub mod branch_audit;
pub mod branch_graph_surface;
pub mod build_discovery_surface;
pub mod build_proof_log;
pub mod build_surface;
pub mod chat_surface;
pub mod chat_tui;
pub mod cli_args;
pub mod cli_json;
pub mod command_manifest;
pub mod coordination_ledger;
pub mod coordination_log;
pub mod coordination_metrics;
pub mod coordination_surface;
pub mod constitutional_court_surface;
pub mod crate_audit;
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
pub mod discovery_surface;
pub mod doctor;
pub mod external_audit;
pub mod federation_surface;
pub mod forensics_surface;
pub mod fs_json;
pub mod host_exec_alias_surface;
pub mod host_profile_selection;
pub mod host_surface;
pub mod idea_ledger;
pub mod idea_surface;
pub(crate) mod hub_surface;
pub(crate) mod inference_surface;
pub mod invoice_log;
pub mod knowledge_surface;
pub mod mask;
#[cfg(feature = "topology_write")]
pub mod topology_write;

pub mod daemon_surface;
pub mod supervise_surface;
pub mod mcp_surface;
pub mod network_audit;
pub mod network_notify_surface;
pub mod network_surface;
pub mod node_host_audit;
pub mod node_surface;
pub mod operator_doctor;
pub mod operator_errors;
pub mod operator_help;
pub mod operator_status;
pub mod persistence_audit;
pub mod privilege_surface;
pub mod queue_surface;
pub mod root_dispatch;
pub mod runtime_paths;
pub mod runtime_suite;
pub mod runtime_surface;
pub mod rust_source;
pub mod secret_surface;
pub mod secret_use_surface;
pub mod session_surface;
pub mod source_surface;
pub mod storage_surface;
pub mod surface_registry;
pub mod task_ledger;
pub mod task_surface;
pub mod transport_surface;
pub mod version_surface;
pub mod wiki_surface;
pub mod workload_observation;

pub use cli_args::Cli;
pub(crate) use cli_args::Command;

use crate::host_exec_alias_surface::maybe_run_host_exec_alias;
use crate::root_dispatch::{execute_root_command, print_cli_help};
use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};

pub async fn run_main() -> ExitCode {
    if let Some(exit_code) = maybe_run_host_exec_alias() {
        return exit_code;
    }

    let cli = Cli::parse();
    match cli.command {
        None => {
            let mut command = Cli::command();
            print_cli_help(&mut command)
        }
        Some(command) => execute_root_command(command).await,
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
