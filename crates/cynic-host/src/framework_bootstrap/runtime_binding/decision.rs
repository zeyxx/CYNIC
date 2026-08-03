use super::types::{
    RUNTIME_BINDING_PLAN_SCHEMA, RuntimeBindingAction, RuntimeBindingObservation,
    RuntimeBindingPlan, RuntimeBindingSpec,
};

pub fn plan_runtime_binding(
    spec: &RuntimeBindingSpec,
    observed: &RuntimeBindingObservation,
) -> RuntimeBindingPlan {
    let mut blockers = Vec::new();
    if !observed.service_unit_present {
        blockers.push("runtime_binding_service_unit_missing".to_string());
    } else if observed.service_unit_consumes_environment != Some(true) {
        blockers.push("runtime_binding_service_unit_ignores_environment".to_string());
    }
    let converged = blockers.is_empty()
        && observed.environment_binding_matches == Some(true)
        && observed.endpoint_reachable == Some(true);
    let actions = if blockers.is_empty() && !converged {
        let mut actions = Vec::new();
        if observed.environment_binding_matches != Some(true) {
            actions.push(RuntimeBindingAction::WriteEnvironmentProjection);
        }
        actions.push(RuntimeBindingAction::RestartService);
        actions
    } else {
        Vec::new()
    };
    let status = if !blockers.is_empty() {
        "blocked"
    } else if converged {
        "converged"
    } else {
        "planned"
    };
    RuntimeBindingPlan {
        schema: RUNTIME_BINDING_PLAN_SCHEMA,
        status,
        mutates: false,
        authority_role: spec.authority_role.clone(),
        service_name: spec.service_name.clone(),
        unit_name: spec.unit_name.clone(),
        endpoint_url: spec.endpoint_url(),
        network_plane: spec.network_plane.clone(),
        actions,
        blockers,
        residual_unknowns: observed.unknowns.clone(),
        required_proofs: vec![
            "kernel_environment_binding_read_back",
            "kernel_service_restart_exit_status",
            "kernel_endpoint_socket_read_back",
        ],
        rollback_intent: "restore the previous environment projection, restart the same declared service, and retain endpoint reachability as unknown if rollback read-back fails",
    }
}
