use crate::{AgentBudget, AgentCapabilityGrant, SecurityEvidenceState, digest_json};

use super::*;

const NOW: u64 = 20_000;

fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn capability(id: &str, mutates: bool) -> AgentCapabilityGrant {
    AgentCapabilityGrant {
        id: id.to_string(),
        mutates,
    }
}

fn authority() -> AgentSessionAuthority {
    let mut authority = AgentSessionAuthority {
        schema: AGENT_SESSION_AUTHORITY_SCHEMA.to_string(),
        revision: 4,
        authority_digest: String::new(),
        peer_observation_max_age_ms: 2_000,
        enrollments: vec![AgentSessionEnrollment {
            principal: "uid:1000".to_string(),
            node_id: "node-a".to_string(),
            transport: AgentSessionTransport::LocalUnixPeer,
            credential_kind: AgentSessionCredentialKind::LocalPeerBinding,
            max_ttl_ms: 60_000,
            agent_types: vec!["codex".to_string()],
            capabilities: vec![
                capability("coord.read", false),
                capability("source.claim", true),
            ],
        }],
    };
    authority.authority_digest = agent_session_authority_digest(&authority).unwrap();
    authority
}

fn peer() -> AgentSessionPeerObservation {
    AgentSessionPeerObservation {
        schema: AGENT_SESSION_PEER_OBSERVATION_SCHEMA.to_string(),
        observed_at_unix_ms: NOW,
        node_id: "node-a".to_string(),
        transport: AgentSessionTransport::LocalUnixPeer,
        principal: Some("uid:1000".to_string()),
        process_id: Some(1234),
        user_id: Some(1000),
        group_id: Some(1000),
        transport_binding_sha256: Some(digest('d')),
        peer_identity: SecurityEvidenceState::Proven,
        evidence_refs: vec!["linux_so_peercred:pid=1234:uid=1000:gid=1000".to_string()],
        unknowns: Vec::new(),
    }
}

fn launch() -> AgentRootLaunchRequest {
    AgentRootLaunchRequest {
        schema: AGENT_ROOT_LAUNCH_REQUEST_SCHEMA.to_string(),
        request_id: "launch-1".to_string(),
        agent_id: "codex:node-a:boot-a:session-a".to_string(),
        agent_type: "codex".to_string(),
        node_id: "node-a".to_string(),
        objective: "audit the bounded source target".to_string(),
        requested_at_unix_ms: NOW,
        requested_ttl_ms: 30_000,
        transport: AgentSessionTransport::LocalUnixPeer,
        credential_kind: AgentSessionCredentialKind::LocalPeerBinding,
        capabilities: vec![
            capability("coord.read", false),
            capability("source.claim", true),
        ],
        budget: AgentBudget {
            context_tokens: 40_000,
            actions: 20,
            children: 0,
            wall_time_ms: 30_000,
        },
        expected_proofs: vec!["coordination_read_back".to_string()],
        confirmation_key_sha256: None,
    }
}

#[test]
fn enrolled_local_peer_gets_a_bounded_root_session_plan() {
    let plan = compose_root_agent_launch(&launch(), &authority(), &peer(), NOW);
    assert!(plan.admitted(), "{plan:?}");
    let delegation = plan.delegation.as_ref().unwrap();
    let request = plan.session_request.as_ref().unwrap();
    assert_eq!(request.delegation_digest, digest_json(delegation).unwrap());
    assert_eq!(
        delegation.authority_snapshot.digest,
        authority().authority_digest
    );
    assert_eq!(delegation.depth, 0);
    assert_eq!(delegation.capabilities, launch().capabilities);
}

#[test]
fn root_launch_rejects_an_agent_type_outside_enrollment() {
    let mut request = launch();
    request.agent_type = "unbounded-supervisor".to_string();
    let plan = compose_root_agent_launch(&request, &authority(), &peer(), NOW);
    assert!(!plan.admitted());
    assert!(
        plan.blockers
            .contains(&"agent_root_launch_agent_type_not_enrolled".to_string())
    );
}

#[test]
fn root_launch_rejects_capability_amplification() {
    let mut request = launch();
    request.capabilities.push(capability("host.root", true));
    let plan = compose_root_agent_launch(&request, &authority(), &peer(), NOW);
    assert!(!plan.admitted());
    assert!(
        plan.blockers.iter().any(|blocker| {
            blocker == "agent_session_capability_amplifies_enrollment:host.root"
        })
    );
}

#[test]
fn local_transport_rejects_a_jwt_credential() {
    let mut authority = authority();
    authority.enrollments[0].credential_kind = AgentSessionCredentialKind::SignedJwt;
    let mut request = launch();
    request.credential_kind = AgentSessionCredentialKind::SignedJwt;
    request.confirmation_key_sha256 = Some(digest('b'));
    let plan = compose_root_agent_launch(&request, &authority, &peer(), NOW);
    assert!(!plan.admitted());
    assert!(
        plan.blockers
            .contains(&"agent_session_jwt_requires_federation_boundary".to_string())
    );
}

#[test]
fn durable_local_record_contains_a_binding_digest_not_a_secret() {
    let record = AgentSessionRecord {
        schema: AGENT_SESSION_RECORD_SCHEMA.to_string(),
        session_id: "session-1".to_string(),
        request_id: "launch-1".to_string(),
        agent_id: "agent-1".to_string(),
        agent_type: Some("codex".to_string()),
        node_id: "node-a".to_string(),
        delegation_digest: digest('c'),
        authority_revision: 4,
        credential_kind: AgentSessionCredentialKind::LocalPeerBinding,
        credential_digest: None,
        confirmation_key_sha256: None,
        transport_binding_sha256: Some(digest('d')),
        issued_at_unix_ms: NOW,
        expires_at_unix_ms: NOW + 30_000,
        generation: 1,
        state: AgentSessionState::Active,
        revoked_at_unix_ms: None,
        revocation_reason: None,
        capabilities: vec![capability("coord.read", false)],
        evidence_refs: vec!["linux_so_peercred:pid=1234:uid=1000:gid=1000".to_string()],
    };
    assert!(validate_agent_session_record(&record).is_empty());
}

#[test]
fn renewal_preserves_authority_and_advances_fencing_generation() {
    let record = AgentSessionRecord {
        schema: AGENT_SESSION_RECORD_SCHEMA.to_string(),
        session_id: "session-1".to_string(),
        request_id: "launch-1".to_string(),
        agent_id: "agent-1".to_string(),
        agent_type: Some("codex".to_string()),
        node_id: "node-a".to_string(),
        delegation_digest: digest('c'),
        authority_revision: 4,
        credential_kind: AgentSessionCredentialKind::LocalPeerBinding,
        credential_digest: None,
        confirmation_key_sha256: None,
        transport_binding_sha256: Some(digest('d')),
        issued_at_unix_ms: NOW - 1_000,
        expires_at_unix_ms: NOW + 30_000,
        generation: 3,
        state: AgentSessionState::Active,
        revoked_at_unix_ms: None,
        revocation_reason: None,
        capabilities: vec![capability("coord.read", false)],
        evidence_refs: vec!["session-issue:test".to_string()],
    };
    let request = AgentSessionRenewalRequest {
        schema: AGENT_SESSION_RENEWAL_REQUEST_SCHEMA.to_string(),
        request_id: "renew-1".to_string(),
        requested_at_unix_ms: NOW,
        requested_ttl_ms: 20_000,
    };
    let plan = reconcile_agent_session_renewal(&request, &authority(), &record, &peer(), NOW);
    assert!(plan.admitted());
    assert_eq!(plan.replacement_generation, Some(4));
    assert_eq!(plan.expires_at_unix_ms, Some(NOW + 20_000));
}

#[test]
fn renewal_rejects_transport_rebinding() {
    let mut record = AgentSessionRecord {
        schema: AGENT_SESSION_RECORD_SCHEMA.to_string(),
        session_id: "session-1".to_string(),
        request_id: "launch-1".to_string(),
        agent_id: "agent-1".to_string(),
        agent_type: Some("codex".to_string()),
        node_id: "node-a".to_string(),
        delegation_digest: digest('c'),
        authority_revision: 4,
        credential_kind: AgentSessionCredentialKind::LocalPeerBinding,
        credential_digest: None,
        confirmation_key_sha256: None,
        transport_binding_sha256: Some(digest('d')),
        issued_at_unix_ms: NOW - 1_000,
        expires_at_unix_ms: NOW + 30_000,
        generation: 1,
        state: AgentSessionState::Active,
        revoked_at_unix_ms: None,
        revocation_reason: None,
        capabilities: vec![capability("coord.read", false)],
        evidence_refs: vec!["session-issue:test".to_string()],
    };
    record.transport_binding_sha256 = Some(digest('e'));
    let request = AgentSessionRenewalRequest {
        schema: AGENT_SESSION_RENEWAL_REQUEST_SCHEMA.to_string(),
        request_id: "renew-1".to_string(),
        requested_at_unix_ms: NOW,
        requested_ttl_ms: 20_000,
    };
    let plan = reconcile_agent_session_renewal(&request, &authority(), &record, &peer(), NOW);
    assert!(!plan.admitted());
    assert!(
        plan.blockers
            .contains(&"agent_session_renewal_transport_changed".to_string())
    );
}
