use std::process::ExitCode;

use cynic_host::adapters::local_operator_transport::LocalOperatorTransportObserver;
use cynic_host::adapters::proxmox_substrate::ProxmoxSshSubstrateAdapter;
use cynic_host::ports::NodeSubstrateObservationPort;
use cynic_host::registry::{
    resolve_node_substrate_desired_state, resolve_provider_control_transport,
};
use cynic_host::update_lifecycle::LocalArgvRunner;

use crate::{emit_json, fail};

pub(super) fn execute_substrate_plan(
    json: bool,
    registry: Option<String>,
    node: String,
    observation: Option<String>,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let assessment = match observation {
        Some(observation) => cynic_host::node_substrate::assess_node_substrate(
            registry.as_deref(),
            &node,
            Some(&observation),
        ),
        None => assess_live_proxmox_substrate(registry.as_deref(), &node),
    };
    match assessment {
        Ok(report) => emit_json(&report, report.status != "converged"),
        Err(error) => fail(error),
    }
}

fn assess_live_proxmox_substrate(
    registry: Option<&str>,
    node: &str,
) -> Result<cynic_host::node_substrate::NodeSubstrateAssessment, String> {
    let desired = resolve_node_substrate_desired_state(registry, node)?;
    let Some(provider) = desired.provider.as_deref() else {
        return Err("provider_resource_ref_missing".to_string());
    };
    if provider != "proxmox" {
        return Err(format!("provider_substrate_adapter_unavailable:{provider}"));
    }
    let transport = resolve_provider_control_transport(registry, provider)?;
    require_fresh_trusted_ssh_route(&transport.ssh_target)?;
    let runner = LocalArgvRunner;
    let adapter = ProxmoxSshSubstrateAdapter::new(&runner, transport.ssh_target)?;
    cynic_host::node_substrate::assess_live_node_substrate(registry, node, &adapter)
}

pub(super) fn execute_substrate_apply(
    json: bool,
    registry: Option<String>,
    node: String,
    yes: bool,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let desired = match resolve_node_substrate_desired_state(registry.as_deref(), &node) {
        Ok(desired) => desired,
        Err(error) => return fail(error),
    };
    let Some(provider) = desired.provider.as_deref() else {
        return fail("provider_resource_ref_missing".to_string());
    };
    if provider != "proxmox" {
        return fail(format!("provider_substrate_adapter_unavailable:{provider}"));
    }
    let transport = match resolve_provider_control_transport(registry.as_deref(), provider) {
        Ok(transport) => transport,
        Err(error) => return fail(error),
    };
    if let Err(error) = require_fresh_trusted_ssh_route(&transport.ssh_target) {
        return fail(error);
    }
    let runner = LocalArgvRunner;
    let adapter = match ProxmoxSshSubstrateAdapter::new(&runner, transport.ssh_target) {
        Ok(adapter) => adapter,
        Err(error) => return fail(error),
    };
    let observed = match adapter.observe_node_substrate(&desired) {
        Ok(observed) => observed,
        Err(error) => return fail(error),
    };
    let initial = cynic_host::node_substrate::assess_observed_node_substrate(desired, observed);
    let report = cynic_host::node_substrate::apply_node_substrate(initial, yes, &adapter, &adapter);
    emit_json(
        &report,
        !matches!(
            report.status,
            "applied" | "applied_pending_management_proof"
        ),
    )
}

fn require_fresh_trusted_ssh_route(target: &str) -> Result<(), String> {
    let observer = LocalOperatorTransportObserver;
    let inspection = cynic_host::transport::inspect_ssh_target(&observer, target);
    let store_path = cynic_host::ssh_trust::default_ssh_trust_store_path();
    let store = cynic_host::ssh_trust::load_ssh_trust_store(&store_path)?;
    let verification = cynic_host::ssh_trust::verify_ssh_target_trust(&inspection, &store);
    if verification.status != "trusted_match" {
        return Err(format!(
            "provider_control_ssh_trust_required:{}:{}",
            verification.target, verification.status
        ));
    }
    Ok(())
}
