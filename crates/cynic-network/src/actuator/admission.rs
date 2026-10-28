use crate::actuator::types::{
    NetworkActuationPlan, NetworkActuationRequest, NetworkActuationWireRequest,
    NETWORK_ACTUATION_REQUEST_SCHEMA,
};
use cynic_protocol::{
    PolicyAdmissionReport, PolicyAuthority, PolicyDecision, PolicyIntentRef, PolicyScope,
    PolicySetRef,
};

pub fn admit_network_actuation_plan(
    plan: &NetworkActuationPlan,
    actor: impl Into<String>,
) -> PolicyAdmissionReport {
    let digest = cynic_protocol::digest_json(plan).unwrap_or_else(|_| "sha256:unavailable".to_string());
    let mutation = cynic_protocol::MutationContract {
        kind: "network_actuation".to_string(),
        scope: PolicyScope {
            domain: "network".to_string(),
            target: plan.target.clone(),
        },
        mutates: true,
        requires_privilege: true,
        requires_explicit_apply: true,
        required_authority: PolicyAuthority::HumanOperator,
        preconditions: plan.preconditions.clone(),
        rollback_intent: plan.rollback_intent.clone(),
        expected_proofs: plan.expected_proofs.clone(),
    };
    PolicyAdmissionReport::new(
        PolicySetRef {
            id: "cynic.host.network_actuation".to_string(),
            version: "v1".to_string(),
            digest: format!(
                "sha256:{}",
                cynic_protocol::digest_json(&"cynic.host.network_actuation.v1")
                    .unwrap_or_else(|_| "unavailable".to_string())
            ),
        },
        PolicyIntentRef {
            id: plan.id.clone(),
            kind: mutation.kind.clone(),
            actor: actor.into(),
            subject: plan.target.clone(),
            digest: digest.clone(),
            plan_digest: digest,
        },
        mutation.clone(),
        PolicyDecision::Allow {
            required_authority: mutation.required_authority,
            required_proofs: mutation.expected_proofs.clone(),
        },
    )
}

pub fn gate_network_actuation_plan(
    plan: &NetworkActuationPlan,
    admission: &PolicyAdmissionReport,
    authorization: &cynic_protocol::ExecutionAuthorization,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if admission.intent.plan_digest != cynic_protocol::digest_json(plan).unwrap_or_else(|_| "sha256:unavailable".to_string()) {
        blockers.push("plan_digest_mismatch".to_string());
    }
    if !admission.admitted() {
        blockers.push("policy_decision_not_allow".to_string());
    }
    if authorization.authority != PolicyAuthority::HumanOperator {
        blockers.push("authority_insufficient".to_string());
    }
    if !authorization.explicit_apply {
        blockers.push("explicit_apply_required".to_string());
    }
    for required in &plan.preconditions {
        if !authorization.satisfied_preconditions.contains(required) {
            blockers.push(format!("precondition_unsatisfied:{required}"));
        }
    }
    blockers
}

pub fn gate_network_actuation_request(request: &NetworkActuationRequest) -> Vec<String> {
    let mut blockers = if request.schema == NETWORK_ACTUATION_REQUEST_SCHEMA {
        Vec::new()
    } else {
        vec!["request_schema_invalid".to_string()]
    };
    blockers.extend(gate_network_actuation_plan(
        &request.plan,
        &request.admission,
        &request.authorization,
    ));
    blockers
}