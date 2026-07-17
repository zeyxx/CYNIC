#![cfg_attr(test, allow(dead_code, clippy::expect_used))]
#![allow(
    clippy::needless_borrows_for_generic_args,
    clippy::uninlined_format_args,
    clippy::unwrap_used,
    clippy::collapsible_if,
    clippy::implicit_clone,
    clippy::needless_pass_by_value,
    clippy::single_match,
    clippy::unused_self,
    missing_debug_implementations,
    async_fn_in_trait,
    clippy::too_many_arguments,
    clippy::redundant_clone,
    clippy::manual_contains,
    clippy::needless_borrow,
    clippy::print_stdout
)]

pub mod adapters;
mod build_attestation;
mod cli;
mod dns_activation_status;
mod dns_bundle;
mod dns_contract;
mod dns_plan;
pub mod domain;
pub mod fleet_inventory;
pub mod fleet_observability;
pub mod inference_node_ops;
pub mod inference_operations;
pub mod mesh_registry;
mod network_activation_chain;
mod network_actuator;
mod network_actuator_contract;
mod network_actuator_status;
mod network_command_surface;
mod network_diagnostic_surface;
mod network_evidence;
mod network_host_substrate;
mod network_mesh_apply_plan;
mod network_mesh_bundle;
mod network_mesh_plan;
mod network_repair;
mod network_root_policy;
mod network_runtime_proof;
mod network_runtime_status;
mod network_shadow_proof;
mod observe;
pub mod ports;
mod provider_fallback_hosts;
mod provider_fallback_status;
pub mod usecases;
pub mod verify;
pub mod websocket;

pub use cli::NetworkCommands;

use std::process::ExitCode;

pub fn build_network_runtime_status_value(
    registry: &str,
    resolve_dns: bool,
    check_provider_connectivity: bool,
    provider_hosts_path: &str,
    hosts_path: &str,
    corefile_path: &str,
    service_path: &str,
    resolved_dropin_path: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let report = network_runtime_status::build_network_runtime_status(
        registry,
        resolve_dns,
        check_provider_connectivity,
        provider_hosts_path,
        hosts_path,
        corefile_path,
        service_path,
        resolved_dropin_path,
    )?;
    Ok(serde_json::to_value(report)?)
}

pub fn execute_observe_command(
    registry: &str,
    fail_on_warning: bool,
    fail_on_critical: bool,
) -> ExitCode {
    usecases::network_surfaces_uc::execute_observe(registry, fail_on_warning, fail_on_critical)
}

pub fn execute_network_command(command: &NetworkCommands) -> ExitCode {
    usecases::network_surfaces_uc::execute_network_command(command)
}
