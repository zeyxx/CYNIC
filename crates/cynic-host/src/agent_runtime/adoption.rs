//! Pure ownership reconciliation for externally discovered agent runtimes.

use serde::Serialize;

use super::{
    AgentRuntimeActivation, AgentRuntimeObservation, AgentRuntimeOwnership,
    AgentRuntimeOwnershipState, CapabilityNode, CapabilityState,
};

pub const AGENT_RUNTIME_ADOPTION_PLAN_SCHEMA: &str = "cynic.agent_runtime_adoption_plan.v1";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntimeAdoptionDecision {
    DiscoveryRequired,
    SideBySideProofRequired,
    RuntimeOwnedNotReady,
    ReadyForExplicitCutover,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRuntimeAdoptionPlan {
    pub schema: &'static str,
    pub runtime_id: &'static str,
    pub desired_principal: String,
    pub desired_state: AgentRuntimeAdoptionDesiredState,
    pub ownership: AgentRuntimeOwnership,
    pub decision: AgentRuntimeAdoptionDecision,
    pub mutates_host: bool,
    pub preflight: Vec<&'static str>,
    pub bounded_action: AgentRuntimeBoundedAction,
    pub read_back: Vec<&'static str>,
    pub rollback_intent: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRuntimeAdoptionDesiredState {
    pub process_owned_by_runtime_principal: bool,
    pub read_only_supervision_only: bool,
    pub declared_supervisor_required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRuntimeBoundedAction {
    pub action: String,
    pub requires_operator_consent: bool,
    pub argv: Vec<String>,
    pub stop_condition: String,
}

/// Reconcile a read-only report with process ownership. This is deliberately
/// a projection: it cannot start, stop, reconfigure, or adopt a process.
pub fn apply_runtime_ownership(
    runtime: &mut AgentRuntimeObservation,
    ownership: AgentRuntimeOwnership,
) {
    let state = match ownership.state {
        AgentRuntimeOwnershipState::RuntimeOwned => CapabilityState::Observed,
        AgentRuntimeOwnershipState::OperatorLocal
        | AgentRuntimeOwnershipState::ExternalUnadopted => CapabilityState::Degraded,
        AgentRuntimeOwnershipState::Unknown => CapabilityState::Unknown,
    };
    runtime.graph.push(CapabilityNode {
        id: "runtime_ownership",
        state,
        depends_on: vec!["runtime_principal", "binary_install_provenance"],
        retained_capabilities: matches!(ownership.state, AgentRuntimeOwnershipState::RuntimeOwned)
            .then_some("runtime_owned_observation")
            .into_iter()
            .collect(),
        reason: ownership.reason.clone(),
    });
    let gateway_ready = runtime.graph.iter().any(|node| {
        node.id == "gateway_service" && matches!(node.state, CapabilityState::Observed)
    });
    if matches!(ownership.state, AgentRuntimeOwnershipState::RuntimeOwned) && gateway_ready {
        runtime.supervision.activation = AgentRuntimeActivation::SupervisedReadOnly;
        if !runtime
            .supervision
            .retained_operations
            .contains(&"run_observation")
        {
            runtime
                .supervision
                .retained_operations
                .push("run_observation");
        }
    } else {
        runtime.supervision.activation = AgentRuntimeActivation::Discovered;
        runtime
            .supervision
            .retained_operations
            .retain(|operation| *operation != "run_observation");
        runtime
            .supervision
            .blocked_operations
            .push("runtime_process_adoption");
        runtime
            .supervision
            .blockers
            .push(format!("runtime_ownership:{}", ownership.reason));
    }
}

pub fn plan_runtime_adoption(
    runtime_id: &'static str,
    desired_principal: String,
    ownership: AgentRuntimeOwnership,
    gateway_ready: bool,
) -> AgentRuntimeAdoptionPlan {
    let decision = match ownership.state {
        AgentRuntimeOwnershipState::Unknown => AgentRuntimeAdoptionDecision::DiscoveryRequired,
        AgentRuntimeOwnershipState::OperatorLocal
        | AgentRuntimeOwnershipState::ExternalUnadopted => {
            AgentRuntimeAdoptionDecision::SideBySideProofRequired
        }
        AgentRuntimeOwnershipState::RuntimeOwned if gateway_ready => {
            AgentRuntimeAdoptionDecision::ReadyForExplicitCutover
        }
        AgentRuntimeOwnershipState::RuntimeOwned => {
            AgentRuntimeAdoptionDecision::RuntimeOwnedNotReady
        }
    };
    let preflight = match decision {
        AgentRuntimeAdoptionDecision::DiscoveryRequired => vec![
            "observe_external_runtime",
            "select_runtime_process_without_name_search",
            "prove_runtime_principal_identity",
        ],
        AgentRuntimeAdoptionDecision::SideBySideProofRequired => vec![
            "retain_operator_runtime_without_mutation",
            "declare_cynic_owned_candidate",
            "prove_candidate_process_identity_and_gateway_health",
            "obtain_read_back_before_explicit_cutover",
        ],
        AgentRuntimeAdoptionDecision::RuntimeOwnedNotReady => vec![
            "retain_runtime_ownership_evidence",
            "observe_gateway_health_and_capabilities",
            "obtain_read_back_before_explicit_cutover",
        ],
        AgentRuntimeAdoptionDecision::ReadyForExplicitCutover => vec![
            "present_explicit_cutover_request",
            "apply_only_after_operator_consent",
            "obtain_post_cutover_read_back",
        ],
    };
    let (action, requires_operator_consent, argv, stop_condition) = match decision {
        AgentRuntimeAdoptionDecision::DiscoveryRequired => (
            "observe_then_declare_candidate".to_string(),
            true,
            vec![
                "hermes".to_string(),
                "runtime".to_string(),
                "discover".to_string(),
                "--id".to_string(),
            ],
            "candidate_observed_with_runtime_principal_identity".to_string(),
        ),
        AgentRuntimeAdoptionDecision::SideBySideProofRequired => (
            "side_by_side_adoption".to_string(),
            true,
            vec![
                "hermes".to_string(),
                "runtime".to_string(),
                "adopt".to_string(),
                "--id".to_string(),
                "--principal".to_string(),
            ],
            "candidate_process_identity_and_gateway_health_proven".to_string(),
        ),
        AgentRuntimeAdoptionDecision::RuntimeOwnedNotReady => (
            "runtime_owned_retain_pending".to_string(),
            false,
            vec![
                "hermes".to_string(),
                "runtime".to_string(),
                "status".to_string(),
                "--id".to_string(),
            ],
            "gateway_healthy_with_run_observation_capability".to_string(),
        ),
        AgentRuntimeAdoptionDecision::ReadyForExplicitCutover => (
            "explicit_cutover_activate".to_string(),
            true,
            vec![
                "hermes".to_string(),
                "runtime".to_string(),
                "activate".to_string(),
                "--id".to_string(),
                "--principal".to_string(),
            ],
            "supervision_read_back_confirmed_via_health_endpoint".to_string(),
        ),
    };
    AgentRuntimeAdoptionPlan {
        schema: AGENT_RUNTIME_ADOPTION_PLAN_SCHEMA,
        runtime_id,
        desired_principal: desired_principal.clone(),
        desired_state: AgentRuntimeAdoptionDesiredState {
            process_owned_by_runtime_principal: true,
            read_only_supervision_only: true,
            declared_supervisor_required: true,
        },
        ownership,
        decision,
        mutates_host: false,
        preflight,
        bounded_action: AgentRuntimeBoundedAction {
            action,
            requires_operator_consent,
            argv: argv
                .into_iter()
                .chain(std::iter::once(runtime_id.to_string()))
                .chain(
                    if matches!(
                        decision,
                        AgentRuntimeAdoptionDecision::SideBySideProofRequired
                            | AgentRuntimeAdoptionDecision::ReadyForExplicitCutover
                    ) {
                        vec!["--principal".to_string(), desired_principal]
                    } else {
                        vec![]
                    },
                )
                .collect(),
            stop_condition,
        },
        read_back: vec![
            "observe_selected_process_identity",
            "observe_declared_supervisor",
            "observe_gateway_or_cli_capabilities",
        ],
        rollback_intent: vec![
            "retain_existing_operator_runtime",
            "remove_only_explicitly_created_candidate_after_failed_read_back",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_runtime_requires_side_by_side_proof() {
        let plan = plan_runtime_adoption(
            "hermes-agent",
            "runtime-principal".into(),
            AgentRuntimeOwnership {
                state: AgentRuntimeOwnershipState::ExternalUnadopted,
                adoption_required: true,
                reason: "fixture".into(),
            },
            true,
        );
        assert!(matches!(
            plan.decision,
            AgentRuntimeAdoptionDecision::SideBySideProofRequired
        ));
        assert!(!plan.mutates_host);
    }
}
