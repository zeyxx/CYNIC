use super::{
    authority::agent_session_authority_digest,
    decision::validate_capability_subset,
    record::validate_agent_session_record,
    types::{
        AGENT_SESSION_AUTHORITY_SCHEMA, AGENT_SESSION_PEER_OBSERVATION_SCHEMA,
        AGENT_SESSION_RENEWAL_PLAN_SCHEMA, AGENT_SESSION_RENEWAL_REQUEST_SCHEMA,
        AgentSessionAuthority, AgentSessionPeerObservation, AgentSessionRecord,
        AgentSessionRenewalPlan, AgentSessionRenewalRequest, AgentSessionState, valid_sha256,
    },
};
use crate::SecurityEvidenceState;

pub fn reconcile_agent_session_renewal(
    request: &AgentSessionRenewalRequest,
    authority: &AgentSessionAuthority,
    record: &AgentSessionRecord,
    peer: &AgentSessionPeerObservation,
    now_unix_ms: u64,
) -> AgentSessionRenewalPlan {
    let mut blockers = validate_agent_session_record(record);
    let mut unknowns = peer.unknowns.clone();

    if request.schema != AGENT_SESSION_RENEWAL_REQUEST_SCHEMA
        || request.request_id.trim().is_empty()
        || request.requested_ttl_ms == 0
    {
        blockers.push("agent_session_renewal_request_invalid".to_string());
    }
    if request.requested_at_unix_ms > now_unix_ms
        || now_unix_ms.saturating_sub(request.requested_at_unix_ms)
            > authority.peer_observation_max_age_ms
    {
        blockers.push("agent_session_renewal_request_stale".to_string());
    }
    if authority.schema != AGENT_SESSION_AUTHORITY_SCHEMA
        || authority.revision == 0
        || !valid_sha256(&authority.authority_digest)
        || agent_session_authority_digest(authority).ok().as_deref()
            != Some(authority.authority_digest.as_str())
    {
        blockers.push("agent_session_renewal_authority_invalid".to_string());
    }
    if record.state != AgentSessionState::Active || record.expires_at_unix_ms <= now_unix_ms {
        blockers.push("agent_session_renewal_source_inactive".to_string());
    }
    if record.authority_revision != authority.revision {
        blockers.push("agent_session_renewal_authority_changed".to_string());
    }
    if peer.schema != AGENT_SESSION_PEER_OBSERVATION_SCHEMA
        || peer.node_id != record.node_id
        || peer.peer_identity != SecurityEvidenceState::Proven
        || peer.evidence_refs.is_empty()
    {
        unknowns.push("agent_session_renewal_peer_unproved".to_string());
    }
    if now_unix_ms.saturating_sub(peer.observed_at_unix_ms) > authority.peer_observation_max_age_ms
    {
        unknowns.push("agent_session_renewal_peer_stale".to_string());
    }
    if peer.transport_binding_sha256.as_deref() != record.transport_binding_sha256.as_deref() {
        blockers.push("agent_session_renewal_transport_changed".to_string());
    }

    let enrollment = peer.principal.as_deref().and_then(|principal| {
        authority.enrollments.iter().find(|enrollment| {
            enrollment.principal == principal
                && enrollment.node_id == record.node_id
                && enrollment.credential_kind == record.credential_kind
                && enrollment.transport == peer.transport
        })
    });
    if let Some(enrollment) = enrollment {
        if request.requested_ttl_ms > enrollment.max_ttl_ms {
            blockers.push("agent_session_renewal_ttl_amplification".to_string());
        }
        let Some(agent_type) = record.agent_type.as_deref() else {
            blockers.push("agent_session_renewal_agent_type_missing".to_string());
            return plan(
                request, authority, record, peer, blockers, unknowns, None, None,
            );
        };
        if !enrollment.agent_types.is_empty()
            && !enrollment
                .agent_types
                .iter()
                .any(|allowed| allowed == agent_type)
        {
            blockers.push("agent_session_renewal_agent_type_not_enrolled".to_string());
        }
        validate_capability_subset(
            &record.capabilities,
            &enrollment.capabilities,
            "renewal_enrollment",
            &mut blockers,
        );
    } else {
        blockers.push("agent_session_renewal_principal_not_enrolled".to_string());
    }

    let expires_at = now_unix_ms.checked_add(request.requested_ttl_ms);
    let generation = record.generation.checked_add(1);
    if expires_at.is_none() || generation.is_none() {
        blockers.push("agent_session_renewal_integer_overflow".to_string());
    }
    plan(
        request, authority, record, peer, blockers, unknowns, expires_at, generation,
    )
}

pub fn validate_agent_session_rotation(
    previous: &AgentSessionRecord,
    replacement: &AgentSessionRecord,
    rotated_at_unix_ms: u64,
) -> Vec<String> {
    let mut violations = validate_agent_session_record(replacement);
    if previous.state != AgentSessionState::Active
        || previous.expires_at_unix_ms <= rotated_at_unix_ms
    {
        violations.push("agent_session_rotation_source_inactive".to_string());
    }
    if previous.session_id == replacement.session_id
        || previous.agent_id != replacement.agent_id
        || previous.agent_type != replacement.agent_type
        || previous.node_id != replacement.node_id
        || previous.delegation_digest != replacement.delegation_digest
        || previous.authority_revision != replacement.authority_revision
        || previous.credential_kind != replacement.credential_kind
        || previous.transport_binding_sha256 != replacement.transport_binding_sha256
        || previous.capabilities != replacement.capabilities
    {
        violations.push("agent_session_rotation_authority_amplified".to_string());
    }
    if replacement.state != AgentSessionState::Active
        || replacement.issued_at_unix_ms != rotated_at_unix_ms
        || previous.generation.checked_add(1) != Some(replacement.generation)
        || replacement.revoked_at_unix_ms.is_some()
        || replacement.revocation_reason.is_some()
    {
        violations.push("agent_session_rotation_replacement_invalid".to_string());
    }
    violations.sort();
    violations.dedup();
    violations
}

#[allow(clippy::too_many_arguments)]
fn plan(
    request: &AgentSessionRenewalRequest,
    authority: &AgentSessionAuthority,
    record: &AgentSessionRecord,
    peer: &AgentSessionPeerObservation,
    mut blockers: Vec<String>,
    mut unknowns: Vec<String>,
    expires_at_unix_ms: Option<u64>,
    replacement_generation: Option<u64>,
) -> AgentSessionRenewalPlan {
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
    AgentSessionRenewalPlan {
        schema: AGENT_SESSION_RENEWAL_PLAN_SCHEMA.to_string(),
        status: status.to_string(),
        request_id: request.request_id.clone(),
        previous_session_id: record.session_id.clone(),
        agent_id: record.agent_id.clone(),
        node_id: record.node_id.clone(),
        authority_revision: authority.revision,
        replacement_generation: (status == "planned")
            .then_some(replacement_generation)
            .flatten(),
        expires_at_unix_ms: (status == "planned")
            .then_some(expires_at_unix_ms)
            .flatten(),
        blockers,
        unknowns,
        evidence_refs: peer.evidence_refs.clone(),
    }
}
