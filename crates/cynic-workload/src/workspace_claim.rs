use super::{
    admit_agent_workload,
    types::AgentWorkloadEnvelope,
    workspace_checks::{validate_zone_set, violation},
    workspace_types::{
        AGENT_WORKSPACE_CAPABILITY_ID, AGENT_WORKSPACE_CLAIM_ADMISSION_SCHEMA,
        AGENT_WORKSPACE_CLAIM_INTENT_SCHEMA, AgentWorkspaceClaimAdmission,
        AgentWorkspaceClaimDecision, AgentWorkspaceClaimIntent,
    },
};
use cynic_node_protocol::digest_json;

pub fn admit_agent_workspace_claim(
    workload: &AgentWorkloadEnvelope,
    intent: &AgentWorkspaceClaimIntent,
    max_ttl_ms: u64,
) -> AgentWorkspaceClaimAdmission {
    let mut violations = Vec::new();
    let intent_digest = match digest_json(intent) {
        Ok(digest) => digest,
        Err(_) => {
            violations.push(violation(
                "workspace_claim_intent_not_serializable",
                "workspace claim intent could not be canonicalized",
            ));
            String::new()
        }
    };
    if intent.schema != AGENT_WORKSPACE_CLAIM_INTENT_SCHEMA {
        violations.push(violation(
            "invalid_workspace_claim_intent_schema",
            "workspace claim intent schema is not supported",
        ));
    }
    for (field, value) in [
        ("workload_id", intent.workload_id.as_str()),
        ("repository_id", intent.repository_id.as_str()),
        ("workspace_id", intent.workspace_id.as_str()),
        ("node_id", intent.node_id.as_str()),
        ("agent_id", intent.agent_id.as_str()),
    ] {
        if value.trim().is_empty() {
            violations.push(violation(
                "empty_workspace_claim_field",
                format!("{field} must not be empty"),
            ));
        }
    }
    if intent.requested_ttl_ms == 0
        || max_ttl_ms == 0
        || intent.requested_ttl_ms > max_ttl_ms
        || intent.requested_ttl_ms > workload.delegation.budget.wall_time_ms
    {
        violations.push(violation(
            "workspace_claim_ttl_not_admitted",
            "workspace claim TTL must fit both node policy and delegated wall-time budget",
        ));
    }
    violations.extend(validate_zone_set(&intent.zones));

    if !admit_agent_workload(workload).admitted() {
        violations.push(violation(
            "workspace_claim_workload_not_admitted",
            "workspace claim requires an admitted constitutional workload",
        ));
    }
    if workload.workload_id != intent.workload_id
        || workload.delegation.node_id != intent.node_id
        || workload.delegation.agent_id != intent.agent_id
    {
        violations.push(violation(
            "workspace_claim_identity_mismatch",
            "workspace claim identity must match the workload delegation",
        ));
    }
    if workload.policy_admission.mutation.kind != AGENT_WORKSPACE_CAPABILITY_ID
        || workload.policy_admission.mutation.scope.target != intent.repository_id
    {
        violations.push(violation(
            "workspace_claim_policy_scope_mismatch",
            "workspace claim requires source_change authority for the exact repository",
        ));
    }

    AgentWorkspaceClaimAdmission {
        schema: AGENT_WORKSPACE_CLAIM_ADMISSION_SCHEMA.to_string(),
        workload_id: intent.workload_id.clone(),
        intent_digest,
        max_lifetime_ms: workload.delegation.budget.wall_time_ms.min(max_ttl_ms),
        decision: if violations.is_empty() {
            AgentWorkspaceClaimDecision::Allow
        } else {
            AgentWorkspaceClaimDecision::Deny { violations }
        },
    }
}

pub fn workspace_claim_admission_matches(
    intent: &AgentWorkspaceClaimIntent,
    admission: &AgentWorkspaceClaimAdmission,
) -> bool {
    admission.admitted()
        && admission.workload_id == intent.workload_id
        && digest_json(intent).is_ok_and(|intent_digest| intent_digest == admission.intent_digest)
}
