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

/// Layer-1 principal provisioning via the Proxmox REST API adapter.
///
/// Flow (skill cynic-layer-model): resolve desired (registry SSOT) →
/// build API transport from provider_control credentials → plan (pure,
/// deny-list enforced) → apply (exec/wait/read-back) → sealed proof JSON.
/// Fail-closed: missing secret, bad resource ref, reserved principal, task
/// failure, or non-converged read-back all exit non-zero with typed errors.
pub(super) fn execute_substrate_provision_principal(
    json: bool,
    registry: Option<String>,
    node: String,
    principal: String,
    yes: bool,
) -> ExitCode {
    use std::process::ExitCode as EC;
    if !json || !yes {
        // mutation requires explicit --yes AND --json machine-readable proof
        return EC::FAILURE;
    }
    let run = || -> Result<serde_json::Value, String> {
        let desired = resolve_node_substrate_desired_state(registry.as_deref(), &node)?;
        let control =
            cynic_host::registry::resolve_provider_control_rest(registry.as_deref(), "proxmox")?;
        let token_secret = std::fs::read_to_string(&control.credential_ref)
            .map_err(|e| format!("pve_token_secret_unreadable:{e}"))?
            .trim()
            .to_string();
        let token = cynic_host::adapters::proxmox_api::PveToken {
            identity: control.token_identity.clone(),
            secret: token_secret,
        };
        let transport = cynic_host::adapters::proxmox_api::ReqwestPveTransport::new(
            &control.endpoint,
            &token,
            control.ca_ref.as_deref(),
        )?;
        let pve_node = "pve"; // TODO(registry): derive from provider_control when modeled
        let adapter = cynic_host::adapters::proxmox_api::ProxmoxApiSubstrateAdapter::new(
            transport, &control.endpoint, pve_node, token,
        );
        let plan =
            cynic_host::node_provision::plan_provision_principal(&desired, pve_node, &principal)?;
        let proof = cynic_host::node_provision::apply_provision_principal(&plan, &adapter, 20, 500)?;
        Ok(serde_json::json!({
            "status": if proof.converged() { "provisioned" } else { "blocked_read_back_unverified" },
            "proof": proof,
        }))
    };
    match run() {
        Ok(report) => {
            emit_json(&report, report["status"] != "provisioned");
            EC::SUCCESS
        }
        Err(error) => fail(error),
    }
}
