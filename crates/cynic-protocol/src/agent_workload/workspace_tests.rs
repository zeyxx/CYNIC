use crate::{
    AgentEvidenceLineage, AgentEvidenceRef, AgentWorkloadEnvelope, AgentWorkspaceChange,
    AgentWorkspaceClaimIntent, AgentWorkspaceClaimObservation, AgentWorkspaceDecision,
    AgentWorkspaceObservation, AgentWorkspacePathObservation, AgentWorkspaceRequest,
    AgentWorkspaceZone, AgentWorkspaceZoneKind, LeaseGuard, digest_json,
};

use super::{
    AGENT_WORKSPACE_CLAIM_INTENT_SCHEMA, AGENT_WORKSPACE_OBSERVATION_SCHEMA,
    AGENT_WORKSPACE_REQUEST_SCHEMA, admit_agent_workspace_claim, agent_workspace_target,
    reconcile_agent_workspace, tests::test_envelope, workspace_claim_admission_matches,
    zones_overlap,
};

fn digest(marker: char) -> String {
    format!("sha256:{}", marker.to_string().repeat(64))
}

fn zone(kind: AgentWorkspaceZoneKind, path: &str) -> AgentWorkspaceZone {
    AgentWorkspaceZone {
        kind,
        path: path.to_string(),
    }
}

fn guard(agent_id: &str, workspace_id: &str, token: u64) -> LeaseGuard {
    LeaseGuard {
        agent_id: agent_id.to_string(),
        target: agent_workspace_target("cynic", workspace_id),
        lease_id: format!("lease-{token}"),
        fencing_token: token,
    }
}

fn evidence() -> AgentEvidenceRef {
    AgentEvidenceRef {
        kind: "workspace_observation".to_string(),
        subject: "cynic:workspace-a".to_string(),
        digest: digest('a'),
        lineage: Some(AgentEvidenceLineage {
            producer_id: "node-a".to_string(),
            producer_attestation_digest: digest('b'),
            origin_digests: vec![digest('c')],
            derivation_digests: Vec::new(),
            influence_digests: Vec::new(),
        }),
    }
}

fn changes() -> Vec<AgentWorkspaceChange> {
    vec![AgentWorkspaceChange {
        path: "crates/cynic-protocol/src/lib.rs".to_string(),
        preimage_digest: Some(digest('1')),
        result_digest: Some(digest('2')),
    }]
}

fn workload() -> AgentWorkloadEnvelope {
    let mut workload = test_envelope();
    let input_digest = digest_json(&changes()).unwrap();
    workload.action.argv = vec!["cynic-source-change-apply".to_string()];
    workload.action.input_digest = Some(input_digest);
    workload.delegation.capabilities[0].id = "source_change".to_string();
    workload.policy_admission.intent.kind = "source_change".to_string();
    workload.policy_admission.intent.subject = "cynic".to_string();
    workload.policy_admission.mutation.kind = "source_change".to_string();
    workload.policy_admission.mutation.scope.domain = "source".to_string();
    workload.policy_admission.mutation.scope.target = "cynic".to_string();
    let plan_digest = digest_json(&workload.action).unwrap();
    workload.plan_digest = plan_digest.clone();
    workload.policy_admission.intent.plan_digest = plan_digest;
    workload
}

fn request(workload: &AgentWorkloadEnvelope) -> AgentWorkspaceRequest {
    AgentWorkspaceRequest {
        schema: AGENT_WORKSPACE_REQUEST_SCHEMA.to_string(),
        workload_id: workload.workload_id.clone(),
        repository_id: "cynic".to_string(),
        workspace_id: "workspace-a".to_string(),
        node_id: workload.delegation.node_id.clone(),
        agent_id: workload.delegation.agent_id.clone(),
        base_commit: "commit-a".to_string(),
        change_set_digest: workload.action.input_digest.clone().unwrap(),
        changes: changes(),
        zones: vec![zone(AgentWorkspaceZoneKind::Tree, "crates/cynic-protocol")],
        guard: guard("build-agent-1", "workspace-a", 1),
    }
}

fn claim_intent(workload: &AgentWorkloadEnvelope) -> AgentWorkspaceClaimIntent {
    AgentWorkspaceClaimIntent {
        schema: AGENT_WORKSPACE_CLAIM_INTENT_SCHEMA.to_string(),
        workload_id: workload.workload_id.clone(),
        repository_id: "cynic".to_string(),
        workspace_id: "workspace-a".to_string(),
        node_id: workload.delegation.node_id.clone(),
        agent_id: workload.delegation.agent_id.clone(),
        zones: vec![zone(AgentWorkspaceZoneKind::Tree, "crates/cynic-protocol")],
        requested_ttl_ms: 60_000,
    }
}

fn own_claim(request: &AgentWorkspaceRequest) -> AgentWorkspaceClaimObservation {
    AgentWorkspaceClaimObservation {
        workload_id: request.workload_id.clone(),
        workspace_id: request.workspace_id.clone(),
        node_id: request.node_id.clone(),
        agent_id: request.agent_id.clone(),
        zones: request.zones.clone(),
        guard: request.guard.clone(),
        expires_at_ms: 100,
        hard_expires_at_ms: 100,
    }
}

fn observation(request: &AgentWorkspaceRequest) -> AgentWorkspaceObservation {
    AgentWorkspaceObservation {
        schema: AGENT_WORKSPACE_OBSERVATION_SCHEMA.to_string(),
        repository_id: request.repository_id.clone(),
        workspace_id: request.workspace_id.clone(),
        node_id: request.node_id.clone(),
        exists: true,
        isolated: true,
        head_commit: Some(request.base_commit.clone()),
        path_preimages: request
            .changes
            .iter()
            .map(|change| AgentWorkspacePathObservation {
                path: change.path.clone(),
                digest: change.preimage_digest.clone(),
            })
            .collect(),
        repository_claims: vec![own_claim(request)],
        evidence_refs: vec![evidence()],
    }
}

#[test]
fn isolated_observed_workspace_with_exact_claim_is_ready() {
    let workload = workload();
    let desired = request(&workload);
    assert!(reconcile_agent_workspace(&workload, &desired, &observation(&desired), 10).ready());
}

#[test]
fn source_workload_can_request_a_bounded_workspace_claim() {
    let workload = workload();
    let intent = claim_intent(&workload);
    let admission = admit_agent_workspace_claim(&workload, &intent, 60_000);
    assert!(workspace_claim_admission_matches(&intent, &admission));
}

#[test]
fn workspace_claim_cannot_amplify_repository_scope() {
    let workload = workload();
    let mut intent = claim_intent(&workload);
    intent.repository_id = "other-repository".to_string();
    assert!(!admit_agent_workspace_claim(&workload, &intent, 60_000).admitted());
}

#[test]
fn workspace_claim_ttl_cannot_exceed_delegated_wall_time() {
    let mut workload = workload();
    workload.delegation.budget.wall_time_ms = 1_000;
    let intent = claim_intent(&workload);
    assert!(!admit_agent_workspace_claim(&workload, &intent, 60_000).admitted());
}

#[test]
fn claim_admission_cannot_be_replayed_for_another_zone_set() {
    let workload = workload();
    let intent = claim_intent(&workload);
    let admission = admit_agent_workspace_claim(&workload, &intent, 60_000);
    let mut changed = intent;
    changed.zones = vec![zone(AgentWorkspaceZoneKind::Tree, "crates/cynic-node")];
    assert!(!workspace_claim_admission_matches(&changed, &admission));
}

#[test]
fn parent_and_child_tree_zones_overlap() {
    assert!(zones_overlap(
        &zone(AgentWorkspaceZoneKind::Tree, "crates"),
        &zone(AgentWorkspaceZoneKind::Tree, "crates/cynic-protocol/src")
    ));
    assert!(!zones_overlap(
        &zone(
            AgentWorkspaceZoneKind::Exact,
            "crates/cynic-protocol/src/lib.rs"
        ),
        &zone(
            AgentWorkspaceZoneKind::Exact,
            "crates/cynic-protocol/src/runtime.rs"
        )
    ));
}

#[test]
fn concurrent_overlapping_workload_is_denied_before_mutation() {
    let workload = workload();
    let desired = request(&workload);
    let mut observed = observation(&desired);
    observed
        .repository_claims
        .push(AgentWorkspaceClaimObservation {
            workload_id: "workload-b".to_string(),
            workspace_id: "workspace-b".to_string(),
            node_id: "node-a".to_string(),
            agent_id: "agent-b".to_string(),
            zones: vec![zone(
                AgentWorkspaceZoneKind::Exact,
                "crates/cynic-protocol/src/lib.rs",
            )],
            guard: guard("agent-b", "workspace-b", 2),
            expires_at_ms: 100,
            hard_expires_at_ms: 100,
        });

    assert!(matches!(
        reconcile_agent_workspace(&workload, &desired, &observed, 10).decision,
        AgentWorkspaceDecision::Deny { violations }
            if violations.iter().any(|item| item.code == "workspace_zone_conflict")
    ));
}

#[test]
fn expired_conflicting_claim_does_not_block_reconciliation() {
    let workload = workload();
    let desired = request(&workload);
    let mut observed = observation(&desired);
    observed
        .repository_claims
        .push(AgentWorkspaceClaimObservation {
            workload_id: "workload-b".to_string(),
            workspace_id: "workspace-b".to_string(),
            node_id: "node-a".to_string(),
            agent_id: "agent-b".to_string(),
            zones: desired.zones.clone(),
            guard: guard("agent-b", "workspace-b", 2),
            expires_at_ms: 10,
            hard_expires_at_ms: 100,
        });

    assert!(reconcile_agent_workspace(&workload, &desired, &observed, 10).ready());
}

#[test]
fn shared_or_drifted_workspace_is_denied() {
    let workload = workload();
    let desired = request(&workload);
    let mut observed = observation(&desired);
    observed.isolated = false;
    observed.head_commit = Some("other-commit".to_string());

    assert!(matches!(
        reconcile_agent_workspace(&workload, &desired, &observed, 10).decision,
        AgentWorkspaceDecision::Deny { violations }
            if violations.iter().any(|item| item.code == "workspace_not_isolated")
                && violations
                    .iter()
                    .any(|item| item.code == "workspace_base_commit_mismatch")
    ));
}

#[test]
fn missing_node_proof_remains_unknown() {
    let workload = workload();
    let desired = request(&workload);
    let mut observed = observation(&desired);
    observed.evidence_refs.clear();

    assert!(matches!(
        reconcile_agent_workspace(&workload, &desired, &observed, 10).decision,
        AgentWorkspaceDecision::Unknown { missing_evidence }
            if missing_evidence == vec!["workspace_observation.evidence_refs"]
    ));
}

#[test]
fn missing_path_preimage_remains_unknown() {
    let workload = workload();
    let desired = request(&workload);
    let mut observed = observation(&desired);
    observed.path_preimages.clear();

    assert!(matches!(
        reconcile_agent_workspace(&workload, &desired, &observed, 10).decision,
        AgentWorkspaceDecision::Unknown { missing_evidence }
            if missing_evidence
                == vec!["workspace_preimage:crates/cynic-protocol/src/lib.rs"]
    ));
}

#[test]
fn changed_preimage_is_denied_before_apply() {
    let workload = workload();
    let desired = request(&workload);
    let mut observed = observation(&desired);
    observed.path_preimages[0].digest = Some(digest('9'));

    assert!(matches!(
        reconcile_agent_workspace(&workload, &desired, &observed, 10).decision,
        AgentWorkspaceDecision::Deny { violations }
            if violations
                .iter()
                .any(|item| item.code == "workspace_preimage_mismatch")
    ));
}

#[test]
fn change_set_must_be_bound_by_the_workload_action() {
    let workload = workload();
    let mut desired = request(&workload);
    desired.change_set_digest = digest('f');
    let observed = observation(&desired);

    assert!(matches!(
        reconcile_agent_workspace(&workload, &desired, &observed, 10).decision,
        AgentWorkspaceDecision::Deny { violations }
            if violations
                .iter()
                .any(|item| item.code == "workspace_change_set_not_bound")
    ));
}

#[test]
fn unrelated_workload_capability_cannot_mutate_source() {
    let workload = test_envelope();
    let source_workload = workload();
    let desired = request(&source_workload);
    let observed = observation(&desired);

    assert!(matches!(
        reconcile_agent_workspace(&workload, &desired, &observed, 10).decision,
        AgentWorkspaceDecision::Deny { violations }
            if violations
                .iter()
                .any(|item| item.code == "workspace_policy_scope_mismatch")
    ));
}

#[test]
fn ambiguous_and_redundant_zones_are_denied() {
    let workload = workload();
    let mut desired = request(&workload);
    desired.zones = vec![
        zone(AgentWorkspaceZoneKind::Tree, "crates"),
        zone(AgentWorkspaceZoneKind::Exact, "crates/../Cargo.toml"),
        zone(AgentWorkspaceZoneKind::Exact, "crates/cynic-protocol"),
    ];
    let observed = observation(&desired);

    assert!(matches!(
        reconcile_agent_workspace(&workload, &desired, &observed, 10).decision,
        AgentWorkspaceDecision::Deny { violations }
            if violations.iter().any(|item| item.code == "invalid_workspace_zone")
                && violations
                    .iter()
                    .any(|item| item.code == "overlapping_requested_zones")
    ));
}
