#![allow(clippy::print_stdout, clippy::print_stderr, dead_code)]
//! `cynic-cli` — Portable CYNIC operator CLI library.
//!
//! Conforms to the Library-First Pattern (§472): all domain code, command
//! structures, and dispatch logic live here; `main.rs` is a thin entrypoint.

use clap::{CommandFactory, Parser};
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
pub mod cli_args;
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
pub mod invoice_log;
pub mod judge_surface;
pub mod judgment_surface;
pub mod kernel_boundary_audit;
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
pub mod source_surface;
pub mod surface_audit;
pub mod surface_registry;
pub mod task_surface;
pub mod transport_surface;
pub mod version_surface;
pub mod workload_observation;

pub(crate) use cli_args::{Cli, Command, DepsCommand};

use crate::host_exec_alias_surface::maybe_run_host_exec_alias;
use crate::root_dispatch::{execute_root_command, print_cli_help};
use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};

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
