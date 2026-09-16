use std::process::ExitCode;

use cynic_host::{
    adapters::agent_access::observe_linux_uid,
    agent_runtime::{
        AgentRuntimeInstallOwnership, AgentRuntimeOwnership, AgentRuntimeOwnershipState,
        apply_runtime_ownership, classify_runtime_ownership_with_external_gateway,
        observe_agent_runtimes, observe_linux_process_identity, plan_runtime_adoption,
    },
    runtime_authority::default_runtime_authority_paths,
    cynic_lifecycle::LocalArgvRunner,
};
use cynic_inference::{HermesGatewayService, HermesGatewayState};

use crate::emit_json;

pub(crate) fn execute_agents(json: bool) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let mut report = observe_agent_runtimes(&LocalArgvRunner);
    project_declared_runtime_adoptions(&mut report);
    project_hermes_adoption(&mut report);
    emit_json(&report, false)
}

fn project_declared_runtime_adoptions(
    report: &mut cynic_host::agent_runtime::AgentRuntimeInventoryReport,
) {
    let principal = default_runtime_authority_paths().principal.runtime_user;
    for runtime in report
        .runtimes
        .iter_mut()
        .filter(|runtime| runtime.id != "hermes-agent")
    {
        let ownership = ownership_from_install(runtime.install_provenance.payload_owner);
        let adoption =
            plan_runtime_adoption(runtime.id, principal.clone(), ownership.clone(), false);
        apply_runtime_ownership(runtime, ownership);
        runtime.adoption = Some(adoption);
    }
}

fn ownership_from_install(ownership: AgentRuntimeInstallOwnership) -> AgentRuntimeOwnership {
    match ownership {
        AgentRuntimeInstallOwnership::RuntimePrincipal => AgentRuntimeOwnership {
            state: AgentRuntimeOwnershipState::Unknown,
            adoption_required: false,
            reason: "runtime_payload_owner_observed_but_process_identity_unproven".into(),
        },
        AgentRuntimeInstallOwnership::ExternalPrincipal => AgentRuntimeOwnership {
            state: AgentRuntimeOwnershipState::ExternalUnadopted,
            adoption_required: true,
            reason: "external_payload_owner_requires_adoption_proof".into(),
        },
        AgentRuntimeInstallOwnership::Unknown => AgentRuntimeOwnership {
            state: AgentRuntimeOwnershipState::Unknown,
            adoption_required: false,
            reason: "payload_ownership_unobserved".into(),
        },
    }
}

/// CLI composition only: the host owns reconciliation while the inference
/// adapter supplies the selected CYNIC gateway PID and readiness observation.
fn project_hermes_adoption(report: &mut cynic_host::agent_runtime::AgentRuntimeInventoryReport) {
    let Some(runtime) = report
        .runtimes
        .iter_mut()
        .find(|runtime| runtime.id == "hermes-agent")
    else {
        return;
    };
    let external_gateway_observed = runtime.graph.iter().any(|node| {
        node.id == "gateway_service"
            && matches!(
                node.state,
                cynic_host::agent_runtime::CapabilityState::Observed
            )
    });
    let gateway = HermesGatewayService::from_process().status().ok();
    let principal = default_runtime_authority_paths().principal.runtime_user;
    let process = match (
        gateway.as_ref().and_then(|gateway| gateway.pid),
        observe_linux_uid(&principal),
    ) {
        (Some(pid), Ok(uid)) => Some(observe_linux_process_identity(pid, uid)),
        _ => None,
    };
    let ownership = classify_runtime_ownership_with_external_gateway(
        process.as_ref(),
        external_gateway_observed,
    );
    let gateway_ready = gateway.as_ref().is_some_and(|gateway| {
        gateway.state == HermesGatewayState::Active && gateway.health_ok && gateway.capabilities_ok
    });
    let adoption =
        plan_runtime_adoption("hermes-agent", principal, ownership.clone(), gateway_ready);
    apply_runtime_ownership(runtime, ownership);
    runtime.adoption = Some(adoption);
}
