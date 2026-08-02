use std::collections::BTreeMap;

use crate::{
    AGENT_DELEGATION_ENVELOPE_SCHEMA, AgentCapabilityGrant, AgentDelegationEnvelope,
    AgentRecursionDecision, SecurityEvidenceState, admit_child_delegation, admit_root_delegation,
    digest_json,
};

use super::authority::agent_session_authority_digest;
use super::types::{
    AGENT_SESSION_AUTHORITY_SCHEMA, AGENT_SESSION_PEER_OBSERVATION_SCHEMA,
    AGENT_SESSION_PLAN_SCHEMA, AGENT_SESSION_REQUEST_SCHEMA, AgentSessionAction,
    AgentSessionAuthority, AgentSessionCredentialKind, AgentSessionPeerObservation,
    AgentSessionPlan, AgentSessionRequest, AgentSessionTransport, valid_sha256,
};

pub fn reconcile_agent_session(
    request: &AgentSessionRequest,
    authority: &AgentSessionAuthority,
    peer: &AgentSessionPeerObservation,
    delegation: &AgentDelegationEnvelope,
    parent_delegation: Option<&AgentDelegationEnvelope>,
    now_unix_ms: u64,
) -> AgentSessionPlan {
    let mut blockers = Vec::new();
    let mut unknowns = peer.unknowns.clone();

    validate_inputs(request, authority, peer, &mut blockers, &mut unknowns);
    validate_delegation(
        request,
        delegation,
        parent_delegation,
        &mut blockers,
        &mut unknowns,
    );
    validate_freshness(
        request,
        authority,
        peer,
        now_unix_ms,
        &mut blockers,
        &mut unknowns,
    );

    let enrollment = peer.principal.as_deref().and_then(|principal| {
        authority.enrollments.iter().find(|enrollment| {
            enrollment.principal == principal
                && enrollment.node_id == request.node_id
                && enrollment.transport == request.transport
                && enrollment.credential_kind == request.credential_kind
        })
    });
    let Some(enrollment) = enrollment else {
        blockers.push("agent_session_principal_not_enrolled".to_string());
        return plan(request, authority, blockers, unknowns, peer, None);
    };

    if request.requested_ttl_ms > enrollment.max_ttl_ms {
        blockers.push("agent_session_ttl_amplification".to_string());
    }
    if request.credential_kind != AgentSessionCredentialKind::LocalPeerBinding
        && !request
            .confirmation_key_sha256
            .as_deref()
            .is_some_and(valid_sha256)
    {
        blockers.push("agent_session_confirmation_key_required".to_string());
    }
    if request.credential_kind == AgentSessionCredentialKind::SignedJwt
        && request.transport != AgentSessionTransport::RemoteMutualIdentity
    {
        blockers.push("agent_session_jwt_requires_federation_boundary".to_string());
    }
    validate_capability_subset(
        &request.capabilities,
        &delegation.capabilities,
        "delegation",
        &mut blockers,
    );
    validate_capability_subset(
        &request.capabilities,
        &enrollment.capabilities,
        "enrollment",
        &mut blockers,
    );

    let expires_at = now_unix_ms.checked_add(request.requested_ttl_ms);
    if expires_at.is_none() {
        blockers.push("agent_session_expiry_overflow".to_string());
    }
    plan(request, authority, blockers, unknowns, peer, expires_at)
}

fn validate_inputs(
    request: &AgentSessionRequest,
    authority: &AgentSessionAuthority,
    peer: &AgentSessionPeerObservation,
    blockers: &mut Vec<String>,
    unknowns: &mut Vec<String>,
) {
    if request.schema != AGENT_SESSION_REQUEST_SCHEMA {
        blockers.push("agent_session_request_schema_mismatch".to_string());
    }
    if authority.schema != AGENT_SESSION_AUTHORITY_SCHEMA || authority.revision == 0 {
        blockers.push("agent_session_authority_invalid".to_string());
    }
    if peer.schema != AGENT_SESSION_PEER_OBSERVATION_SCHEMA {
        unknowns.push("agent_session_peer_observation_schema_mismatch".to_string());
    }
    if request.request_id.trim().is_empty()
        || request.agent_id.trim().is_empty()
        || request.node_id.trim().is_empty()
    {
        blockers.push("agent_session_identity_invalid".to_string());
    }
    if !valid_sha256(&authority.authority_digest) {
        blockers.push("agent_session_authority_digest_invalid".to_string());
    } else if agent_session_authority_digest(authority).ok().as_deref()
        != Some(authority.authority_digest.as_str())
    {
        blockers.push("agent_session_authority_digest_mismatch".to_string());
    }
    if !valid_sha256(&request.delegation_digest) {
        blockers.push("agent_session_delegation_digest_invalid".to_string());
    }
}

fn validate_delegation(
    request: &AgentSessionRequest,
    delegation: &AgentDelegationEnvelope,
    parent: Option<&AgentDelegationEnvelope>,
    blockers: &mut Vec<String>,
    unknowns: &mut Vec<String>,
) {
    match digest_json(delegation) {
        Ok(digest) if digest != request.delegation_digest => {
            blockers.push("agent_session_delegation_digest_mismatch".to_string());
        }
        Err(_) => unknowns.push("agent_session_delegation_digest_unavailable".to_string()),
        Ok(_) => {}
    }
    if delegation.schema != AGENT_DELEGATION_ENVELOPE_SCHEMA
        || delegation.agent_id != request.agent_id
        || delegation.node_id != request.node_id
    {
        blockers.push("agent_session_delegation_identity_mismatch".to_string());
    }
    let admission = match parent {
        Some(parent) => admit_child_delegation(parent, delegation),
        None => admit_root_delegation(delegation),
    };
    match admission.decision {
        AgentRecursionDecision::Allow => {}
        AgentRecursionDecision::Deny { .. } => {
            blockers.push("agent_session_delegation_denied".to_string());
        }
        AgentRecursionDecision::Unknown { .. } => {
            unknowns.push("agent_session_delegation_admission_unknown".to_string());
        }
    }
}

fn validate_freshness(
    request: &AgentSessionRequest,
    authority: &AgentSessionAuthority,
    peer: &AgentSessionPeerObservation,
    now_unix_ms: u64,
    blockers: &mut Vec<String>,
    unknowns: &mut Vec<String>,
) {
    if request.requested_at_unix_ms > now_unix_ms {
        blockers.push("agent_session_request_from_future".to_string());
    }
    if now_unix_ms.saturating_sub(request.requested_at_unix_ms)
        > authority.peer_observation_max_age_ms
    {
        blockers.push("agent_session_request_stale".to_string());
    }
    if request.requested_ttl_ms == 0 {
        blockers.push("agent_session_ttl_invalid".to_string());
    }
    if peer.node_id != request.node_id || peer.transport != request.transport {
        blockers.push("agent_session_peer_scope_mismatch".to_string());
    }
    if peer.peer_identity == SecurityEvidenceState::Absent {
        blockers.push("agent_session_peer_identity_absent".to_string());
    } else if peer.peer_identity != SecurityEvidenceState::Proven
        || peer.principal.as_deref().is_none_or(str::is_empty)
        || peer.evidence_refs.is_empty()
    {
        unknowns.push("agent_session_peer_identity_unproved".to_string());
    }
    if now_unix_ms.saturating_sub(peer.observed_at_unix_ms) > authority.peer_observation_max_age_ms
    {
        unknowns.push("agent_session_peer_observation_stale".to_string());
    }
}

pub(super) fn validate_capability_subset(
    requested: &[AgentCapabilityGrant],
    allowed: &[AgentCapabilityGrant],
    authority: &str,
    blockers: &mut Vec<String>,
) {
    let allowed: BTreeMap<&str, bool> = allowed
        .iter()
        .map(|capability| (capability.id.as_str(), capability.mutates))
        .collect();
    for capability in requested {
        match allowed.get(capability.id.as_str()) {
            Some(allows_mutation) if !capability.mutates || *allows_mutation => {}
            _ => blockers.push(format!(
                "agent_session_capability_amplifies_{authority}:{}",
                capability.id
            )),
        }
    }
}

fn plan(
    request: &AgentSessionRequest,
    authority: &AgentSessionAuthority,
    mut blockers: Vec<String>,
    mut unknowns: Vec<String>,
    peer: &AgentSessionPeerObservation,
    expires_at_unix_ms: Option<u64>,
) -> AgentSessionPlan {
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
    AgentSessionPlan {
        schema: AGENT_SESSION_PLAN_SCHEMA.to_string(),
        status: status.to_string(),
        request_id: request.request_id.clone(),
        agent_id: request.agent_id.clone(),
        node_id: request.node_id.clone(),
        authority_revision: authority.revision,
        credential_kind: request.credential_kind,
        expires_at_unix_ms: (status == "planned")
            .then_some(expires_at_unix_ms)
            .flatten(),
        capabilities: request.capabilities.clone(),
        actions: match status {
            "planned" => vec![AgentSessionAction::Issue],
            "unknown" => vec![AgentSessionAction::ObservePeerAgain],
            _ => Vec::new(),
        },
        blockers,
        unknowns,
        evidence_refs: peer.evidence_refs.clone(),
    }
}
