use crate::{
    AGENT_DELEGATION_ENVELOPE_SCHEMA, AgentDelegationEnvelope, AgentEvidenceRef, digest_json,
};

use super::decision::{reconcile_agent_session, validate_capability_subset};
use super::types::{
    AGENT_ROOT_LAUNCH_PLAN_SCHEMA, AGENT_ROOT_LAUNCH_REQUEST_SCHEMA, AGENT_SESSION_REQUEST_SCHEMA,
    AgentRootLaunchPlan, AgentRootLaunchRequest, AgentSessionAuthority,
    AgentSessionPeerObservation, AgentSessionRequest,
};

pub fn compose_root_agent_launch(
    launch: &AgentRootLaunchRequest,
    authority: &AgentSessionAuthority,
    peer: &AgentSessionPeerObservation,
    now_unix_ms: u64,
) -> AgentRootLaunchPlan {
    let mut blockers = Vec::new();
    let mut unknowns = peer.unknowns.clone();
    if launch.schema != AGENT_ROOT_LAUNCH_REQUEST_SCHEMA {
        blockers.push("agent_root_launch_schema_mismatch".to_string());
    }
    if launch.request_id.trim().is_empty()
        || launch.agent_id.trim().is_empty()
        || launch.agent_type.trim().is_empty()
        || launch.node_id.trim().is_empty()
        || launch.objective.trim().is_empty()
    {
        blockers.push("agent_root_launch_identity_invalid".to_string());
    }
    if launch.expected_proofs.is_empty()
        || launch
            .expected_proofs
            .iter()
            .any(|proof| proof.trim().is_empty())
    {
        blockers.push("agent_root_launch_expected_proof_missing".to_string());
    }
    if launch.budget.actions == 0 || launch.budget.wall_time_ms == 0 {
        blockers.push("agent_root_launch_budget_invalid".to_string());
    }
    if peer.node_id != launch.node_id || peer.transport != launch.transport {
        blockers.push("agent_root_launch_peer_scope_mismatch".to_string());
    }

    let enrollment = peer.principal.as_deref().and_then(|principal| {
        authority.enrollments.iter().find(|enrollment| {
            enrollment.principal == principal
                && enrollment.node_id == launch.node_id
                && enrollment.transport == launch.transport
                && enrollment.credential_kind == launch.credential_kind
        })
    });
    let Some(enrollment) = enrollment else {
        blockers.push("agent_root_launch_principal_not_enrolled".to_string());
        return launch_plan(launch, blockers, unknowns, None, None);
    };
    if !enrollment.agent_types.is_empty()
        && !enrollment
            .agent_types
            .iter()
            .any(|agent_type| agent_type == &launch.agent_type)
    {
        blockers.push("agent_root_launch_agent_type_not_enrolled".to_string());
    }
    validate_capability_subset(
        &launch.capabilities,
        &enrollment.capabilities,
        "enrollment",
        &mut blockers,
    );

    let delegation = AgentDelegationEnvelope {
        schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: format!("root:{}", launch.request_id),
        node_id: launch.node_id.clone(),
        agent_id: launch.agent_id.clone(),
        parent_agent_id: None,
        depth: 0,
        objective: launch.objective.clone(),
        authority_snapshot: AgentEvidenceRef {
            kind: "agent_session_authority".to_string(),
            subject: launch.node_id.clone(),
            digest: authority.authority_digest.clone(),
            lineage: None,
        },
        context_refs: Vec::new(),
        capabilities: launch.capabilities.clone(),
        budget: launch.budget.clone(),
        expected_proofs: launch.expected_proofs.clone(),
    };
    let delegation_digest = match digest_json(&delegation) {
        Ok(digest) => digest,
        Err(_) => {
            unknowns.push("agent_root_launch_delegation_digest_unavailable".to_string());
            String::new()
        }
    };
    let request = AgentSessionRequest {
        schema: AGENT_SESSION_REQUEST_SCHEMA.to_string(),
        request_id: launch.request_id.clone(),
        agent_id: launch.agent_id.clone(),
        node_id: launch.node_id.clone(),
        delegation_digest,
        requested_at_unix_ms: launch.requested_at_unix_ms,
        requested_ttl_ms: launch.requested_ttl_ms,
        transport: launch.transport,
        credential_kind: launch.credential_kind,
        capabilities: launch.capabilities.clone(),
        confirmation_key_sha256: launch.confirmation_key_sha256.clone(),
    };
    let session =
        reconcile_agent_session(&request, authority, peer, &delegation, None, now_unix_ms);
    blockers.extend(session.blockers);
    unknowns.extend(session.unknowns);
    let admitted = blockers.is_empty() && unknowns.is_empty();
    launch_plan(
        launch,
        blockers,
        unknowns,
        admitted.then_some(request),
        admitted.then_some(delegation),
    )
}

fn launch_plan(
    launch: &AgentRootLaunchRequest,
    mut blockers: Vec<String>,
    mut unknowns: Vec<String>,
    session_request: Option<AgentSessionRequest>,
    delegation: Option<AgentDelegationEnvelope>,
) -> AgentRootLaunchPlan {
    blockers.sort();
    blockers.dedup();
    unknowns.sort();
    unknowns.dedup();
    let status = if !blockers.is_empty() {
        "blocked"
    } else if !unknowns.is_empty() {
        "unknown"
    } else {
        "planned"
    };
    AgentRootLaunchPlan {
        schema: AGENT_ROOT_LAUNCH_PLAN_SCHEMA.to_string(),
        status: status.to_string(),
        request_id: launch.request_id.clone(),
        blockers,
        unknowns,
        session_request,
        delegation,
    }
}
