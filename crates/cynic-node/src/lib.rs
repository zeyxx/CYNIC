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
pub mod builder_activation;
mod cli;
pub mod config;
mod dns_activation;
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
mod network_surface_inventory;
pub mod node_control_runtime;
mod observe;
pub mod ports;
mod provider_fallback_hosts;
mod provider_fallback_status;
pub mod shutdown;
pub(crate) mod source_change_transaction;
pub mod usecases;
pub mod verify;
pub mod watchdog;
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

pub fn build_dns_activation_plan_value() -> Result<serde_json::Value, String> {
    serde_json::to_value(dns_plan::build_dns_plan())
        .map_err(|error| format!("DNS activation plan serialization failed: {error}"))
}

pub fn build_dns_activation_bundle_value(
    registry: &str,
    output_dir: &str,
) -> Result<serde_json::Value, String> {
    let report = dns_bundle::build_dns_bundle(registry, output_dir)
        .map_err(|error| format!("DNS activation bundle failed: {error}"))?;
    serde_json::to_value(report)
        .map_err(|error| format!("DNS activation bundle serialization failed: {error}"))
}

pub fn build_dns_activation_status_value(
    registry: &str,
    hosts_path: Option<&str>,
    corefile_path: Option<&str>,
    service_path: Option<&str>,
    resolved_dropin_path: Option<&str>,
    resolve_dns: bool,
) -> Result<serde_json::Value, String> {
    let defaults = dns_contract::dns_target_paths();
    let hosts_path = hosts_path.unwrap_or(&defaults.hosts);
    let corefile_path = corefile_path.unwrap_or(&defaults.corefile);
    let service_path = service_path.unwrap_or(&defaults.service);
    let resolved_dropin_path = resolved_dropin_path.unwrap_or(&defaults.resolved_dropin);
    let report = dns_activation_status::build_dns_activation_status(
        registry,
        hosts_path,
        corefile_path,
        service_path,
        resolved_dropin_path,
        resolve_dns,
    )
    .map_err(|error| format!("DNS activation status failed: {error}"))?;
    serde_json::to_value(report)
        .map_err(|error| format!("DNS activation status serialization failed: {error}"))
}

pub fn plan_dns_activation_value(
    registry: &str,
    bundle_dir: &str,
) -> Result<serde_json::Value, String> {
    let spec = dns_activation::resolve_dns_activation_spec(registry, bundle_dir)?;
    let observation = dns_activation::observe_dns_activation(
        &spec,
        &cynic_host::update_lifecycle::LocalArgvRunner,
    );
    serde_json::to_value(dns_activation::plan_dns_activation(&spec, &observation))
        .map_err(|error| format!("DNS activation plan serialization failed: {error}"))
}

pub fn apply_dns_activation_value(
    registry: &str,
    bundle_dir: &str,
    yes: bool,
) -> Result<serde_json::Value, String> {
    dns_activation::apply_dns_activation_value(registry, bundle_dir, yes)
}

pub fn build_provider_fallback_hosts_value(
    registry: &str,
    output_dir: &str,
) -> Result<serde_json::Value, String> {
    let report = provider_fallback_hosts::build_provider_fallback_hosts(registry, output_dir)
        .map_err(|error| format!("provider fallback hosts bundle failed: {error}"))?;
    serde_json::to_value(report)
        .map_err(|error| format!("provider fallback hosts serialization failed: {error}"))
}

pub fn build_provider_fallback_status_value(
    registry: &str,
    hosts_path: &str,
    check_connectivity: bool,
) -> Result<serde_json::Value, String> {
    let report = provider_fallback_status::build_provider_fallback_status(
        registry,
        hosts_path,
        check_connectivity,
    )
    .map_err(|error| format!("provider fallback status failed: {error}"))?;
    serde_json::to_value(report)
        .map_err(|error| format!("provider fallback status serialization failed: {error}"))
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

pub fn execute_network_actuator_apply(registry: &str, profile: &str, apply: bool) -> ExitCode {
    let observation = observe::observe_payload(Some(registry));
    let report = network_actuator::apply_profile(&observation, profile, apply);
    match serde_json::to_string_pretty(&report) {
        Ok(payload) => {
            println!("{payload}");
            if report.status == "completed" {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("network actuator apply serialization failed: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Observe a libvirt-backed harness target and emit the attestation consumed by
/// network execution admission. This surface is read-only: it executes only
/// domain/network/route observation argv through the injected binary paths.
pub fn observe_libvirt_harness_target_attestation(
    context: &cynic_host::runtime_harness::RuntimeHarnessContext,
    target_id: &str,
    transport: &str,
    transport_target: &str,
    expected_network_boundary: &str,
    virsh_bin: &str,
    ip_bin: &str,
) -> Result<cynic_host::runtime_harness::RuntimeHarnessTargetAttestation, String> {
    use crate::ports::harness_target_attestation_port::{
        HarnessTargetAttestationPort, HarnessTargetAttestationRequest,
    };

    let attestor = adapters::libvirt_harness_target_attestor::LibvirtHarnessTargetAttestor::new(
        cynic_host::update_lifecycle::LocalArgvRunner,
        virsh_bin,
        ip_bin,
    );
    attestor.attest_target(
        context,
        &HarnessTargetAttestationRequest {
            target_id: target_id.to_string(),
            provider: "qemu-libvirt".to_string(),
            transport: transport.to_string(),
            transport_target: transport_target.to_string(),
            expected_network_boundary: expected_network_boundary.to_string(),
        },
    )
}
