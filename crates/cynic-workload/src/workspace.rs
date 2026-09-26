use std::collections::BTreeMap;

use cynic_node_protocol::validate_evidence_refs;

use super::{
    admit_agent_workload,
    types::AgentWorkloadEnvelope,
    workspace_checks::{
        agent_workspace_target, normalized_zone_set, own_claim, validate_request, violation,
        zones_overlap,
    },
    workspace_types::{
        AGENT_WORKSPACE_CAPABILITY_ID, AGENT_WORKSPACE_OBSERVATION_SCHEMA,
        AGENT_WORKSPACE_RECONCILIATION_SCHEMA, AgentWorkspaceDecision, AgentWorkspaceObservation,
        AgentWorkspaceReconciliation, AgentWorkspaceRequest,
    },
};

pub fn reconcile_agent_workspace(
    workload: &AgentWorkloadEnvelope,
    request: &AgentWorkspaceRequest,
    observed: &AgentWorkspaceObservation,
    now_ms: u64,
) -> AgentWorkspaceReconciliation {
    let mut violations = validate_request(request);
    let mut missing_evidence = Vec::new();

    if !admit_agent_workload(workload).admitted() {
        violations.push(violation(
            "workspace_workload_not_admitted",
            "workspace reconciliation requires an admitted constitutional workload",
        ));
    }
    if workload.workload_id != request.workload_id
        || workload.delegation.node_id != request.node_id
        || workload.delegation.agent_id != request.agent_id
    {
        violations.push(violation(
            "workspace_workload_identity_mismatch",
            "workspace request identity must match the admitted workload delegation",
        ));
    }
    if workload.action.input_digest.as_deref() != Some(request.change_set_digest.as_str()) {
        violations.push(violation(
            "workspace_change_set_not_bound",
            "the workload bounded action must bind the exact source change-set digest",
        ));
    }
    if workload.policy_admission.mutation.kind != AGENT_WORKSPACE_CAPABILITY_ID
        || workload.policy_admission.mutation.scope.target != request.repository_id
    {
        violations.push(violation(
            "workspace_policy_scope_mismatch",
            "workspace mutation requires source_change authority for the exact repository",
        ));
    }

    if observed.schema != AGENT_WORKSPACE_OBSERVATION_SCHEMA {
        violations.push(violation(
            "invalid_workspace_observation_schema",
            "workspace observation schema is not supported",
        ));
    }
    if observed.repository_id != request.repository_id
        || observed.workspace_id != request.workspace_id
        || observed.node_id != request.node_id
    {
        violations.push(violation(
            "workspace_observation_identity_mismatch",
            "observed repository, workspace, and node must match the request",
        ));
    }
    if observed.evidence_refs.is_empty() {
        missing_evidence.push("workspace_observation.evidence_refs".to_string());
    }
    violations.extend(
        validate_evidence_refs(&observed.evidence_refs)
            .into_iter()
            .map(|item| violation(format!("evidence_{}", item.code), item.message)),
    );
    if !observed.exists {
        violations.push(violation(
            "workspace_absent",
            "the requested workspace does not exist",
        ));
    }
    if observed.exists && !observed.isolated {
        violations.push(violation(
            "workspace_not_isolated",
            "a mutating workload requires a node-observed isolated workspace",
        ));
    }
    match observed.head_commit.as_deref() {
        None | Some("") => missing_evidence.push("workspace_observation.head_commit".to_string()),
        Some(head) if head != request.base_commit => violations.push(violation(
            "workspace_base_commit_mismatch",
            "the observed workspace head differs from the requested base commit",
        )),
        Some(_) => {}
    }
    let mut observed_preimages = BTreeMap::new();
    for preimage in &observed.path_preimages {
        if observed_preimages
            .insert(preimage.path.as_str(), preimage.digest.as_deref())
            .is_some()
        {
            violations.push(violation(
                "duplicate_observed_preimage",
                format!("path '{}' was observed more than once", preimage.path),
            ));
        }
    }
    for change in &request.changes {
        match observed_preimages.get(change.path.as_str()) {
            None => missing_evidence.push(format!("workspace_preimage:{}", change.path)),
            Some(observed_digest) if *observed_digest != change.preimage_digest.as_deref() => {
                violations.push(violation(
                    "workspace_preimage_mismatch",
                    format!(
                        "path '{}' changed after the source transaction was planned",
                        change.path
                    ),
                ));
            }
            Some(_) => {}
        }
    }

    let live_claims = observed
        .repository_claims
        .iter()
        .filter(|claim| claim.expires_at_ms > now_ms)
        .collect::<Vec<_>>();
    for claim in &live_claims {
        let expected_target = agent_workspace_target(&request.repository_id, &claim.workspace_id);
        if claim.guard.agent_id != claim.agent_id
            || claim.guard.target != expected_target
            || claim.guard.lease_id.trim().is_empty()
            || claim.guard.fencing_token == 0
        {
            violations.push(violation(
                "invalid_observed_workspace_claim",
                "a live repository claim is not bound to its agent and workspace",
            ));
        }
    }
    match live_claims.iter().find(|claim| own_claim(request, claim)) {
        Some(claim) if normalized_zone_set(&claim.zones) == normalized_zone_set(&request.zones) => {
        }
        Some(_) => violations.push(violation(
            "workspace_claim_zone_mismatch",
            "the live workload claim does not cover the exact requested zone set",
        )),
        None => violations.push(violation(
            "workspace_claim_missing",
            "no live fenced claim binds this workload to the requested workspace",
        )),
    }

    for claim in live_claims
        .iter()
        .filter(|claim| claim.workload_id != request.workload_id)
    {
        for requested_zone in &request.zones {
            if let Some(claimed_zone) = claim
                .zones
                .iter()
                .find(|claimed_zone| zones_overlap(requested_zone, claimed_zone))
            {
                violations.push(violation(
                    "workspace_zone_conflict",
                    format!(
                        "requested zone '{}' overlaps workload '{}' zone '{}'",
                        requested_zone.path, claim.workload_id, claimed_zone.path
                    ),
                ));
            }
        }
    }

    let decision = if !violations.is_empty() {
        AgentWorkspaceDecision::Deny { violations }
    } else if !missing_evidence.is_empty() {
        missing_evidence.sort();
        missing_evidence.dedup();
        AgentWorkspaceDecision::Unknown { missing_evidence }
    } else {
        AgentWorkspaceDecision::Ready {
            workspace_target: agent_workspace_target(&request.repository_id, &request.workspace_id),
            change_set_digest: request.change_set_digest.clone(),
            zones: normalized_zone_set(&request.zones).into_iter().collect(),
        }
    };
    AgentWorkspaceReconciliation {
        schema: AGENT_WORKSPACE_RECONCILIATION_SCHEMA.to_string(),
        workload_id: request.workload_id.clone(),
        decision,
    }
}
