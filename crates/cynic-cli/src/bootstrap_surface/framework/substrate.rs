use std::process::ExitCode;

use cynic_host::adapters::proxmox_substrate::ProxmoxSshSubstrateAdapter;
use cynic_host::ports::NodeSubstrateObservationPort;
use cynic_host::registry::{
    resolve_node_substrate_desired_state, resolve_provider_control_transport,
};
use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_protocol::reconcile_node_substrate;

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
    match cynic_host::node_substrate::assess_node_substrate(
        registry.as_deref(),
        &node,
        observation.as_deref(),
    ) {
        Ok(report) => emit_json(&report, report.status != "converged"),
        Err(error) => fail(error),
    }
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
    let runner = LocalArgvRunner;
    let adapter = match ProxmoxSshSubstrateAdapter::new(&runner, transport.ssh_target) {
        Ok(adapter) => adapter,
        Err(error) => return fail(error),
    };
    let observed = match adapter.observe_node_substrate(&desired) {
        Ok(observed) => observed,
        Err(error) => return fail(error),
    };
    let decision = reconcile_node_substrate(&desired, &observed);
    let initial = cynic_host::node_substrate::NodeSubstrateAssessment {
        schema: "cynic.node_substrate.assessment.v1",
        status: decision.status.clone(),
        decision,
        desired,
        observed,
    };
    let report = cynic_host::node_substrate::apply_node_substrate(initial, yes, &adapter, &adapter);
    emit_json(
        &report,
        !matches!(
            report.status,
            "applied" | "applied_pending_management_proof"
        ),
    )
}
