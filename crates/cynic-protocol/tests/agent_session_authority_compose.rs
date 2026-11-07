//! Phase Extraire (Cycle 2.2) : `compose_agent_session_authority` et ses
//! données d'entrée vivent en L0 dans `cynic_protocol::agent_session`.
//!
//! Ce test échoue (erreur de compilation) tant que le socle pure
//! `cynic-host/src/agent_access/authority.rs` n'a pas été déplacé vers
//! `cynic-protocol`.

use cynic_protocol::agent_session::{
    AgentEnrollmentPolicy, DEFAULT_AGENT_SESSION_TTL_MS, DEFAULT_PEER_OBSERVATION_MAX_AGE_MS,
    compose_agent_session_authority, default_local_agent_capabilities,
};
use cynic_protocol::{AgentCapabilityGrant, agent_session_authority_digest};

#[test]
fn compose_is_reachable_from_agent_session_l0() {
    let authority = compose_agent_session_authority(
        "node-a",
        1,
        2_000,
        &[AgentEnrollmentPolicy {
            observed_linux_uid: 1000,
            agent_types: vec!["codex".to_string()],
            max_ttl_ms: DEFAULT_AGENT_SESSION_TTL_MS,
            capabilities: vec![AgentCapabilityGrant {
                id: "coord.read".to_string(),
                mutates: false,
            }],
        }],
    )
    .unwrap();

    assert_eq!(authority.enrollments[0].principal, "linux-uid:1000");
    assert_eq!(
        agent_session_authority_digest(&authority).unwrap(),
        authority.authority_digest
    );
    assert_eq!(DEFAULT_PEER_OBSERVATION_MAX_AGE_MS, 30_000);
    assert_eq!(DEFAULT_AGENT_SESSION_TTL_MS, 900_000);
}

#[test]
fn default_capabilities_match_the_local_mcp_session_contract() {
    let capabilities = default_local_agent_capabilities();
    assert_eq!(capabilities.len(), 5);
    assert!(
        capabilities
            .iter()
            .any(|grant| grant.id == "coord.write" && grant.mutates)
    );
    assert!(
        capabilities
            .iter()
            .any(|grant| grant.id == "agent_workload.delivery" && grant.mutates)
    );
}
