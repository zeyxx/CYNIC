use std::collections::BTreeSet;

use cynic_protocol::{
    AGENT_SESSION_AUTHORITY_SCHEMA, AgentCapabilityGrant, AgentSessionAuthority,
    AgentSessionCredentialKind, AgentSessionEnrollment, AgentSessionTransport,
    agent_session_authority_digest,
};
use serde::Serialize;

pub const DEFAULT_AGENT_SESSION_TTL_MS: u64 = 900_000;
pub const DEFAULT_PEER_OBSERVATION_MAX_AGE_MS: u64 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentEnrollmentPolicy {
    pub observed_linux_uid: u32,
    pub agent_types: Vec<String>,
    pub max_ttl_ms: u64,
    pub capabilities: Vec<AgentCapabilityGrant>,
}

pub fn default_local_agent_capabilities() -> Vec<AgentCapabilityGrant> {
    vec![
        capability("coord.read", false),
        capability("coord.write", true),
        capability("observation.write", true),
        capability("runtime.read", false),
        capability("agent_workload.delivery", true),
    ]
}

fn capability(id: &str, mutates: bool) -> AgentCapabilityGrant {
    AgentCapabilityGrant {
        id: id.to_string(),
        mutates,
    }
}

pub fn compose_agent_session_authority(
    node_id: &str,
    revision: u64,
    peer_observation_max_age_ms: u64,
    policies: &[AgentEnrollmentPolicy],
) -> Result<AgentSessionAuthority, String> {
    if node_id.trim().is_empty()
        || revision == 0
        || peer_observation_max_age_ms == 0
        || policies.is_empty()
    {
        return Err("agent enrollment authority inputs are incomplete".to_string());
    }
    let mut principals = BTreeSet::new();
    let mut enrollments = Vec::with_capacity(policies.len());
    for policy in policies {
        let principal = format!("linux-uid:{}", policy.observed_linux_uid);
        if !principals.insert(principal.clone())
            || policy.max_ttl_ms == 0
            || policy.agent_types.is_empty()
            || policy.agent_types.iter().any(|kind| kind.trim().is_empty())
            || policy
                .capabilities
                .iter()
                .any(|grant| grant.id.trim().is_empty())
        {
            return Err(format!(
                "agent enrollment policy is invalid for `{principal}`"
            ));
        }
        enrollments.push(AgentSessionEnrollment {
            principal,
            node_id: node_id.to_string(),
            transport: AgentSessionTransport::LocalUnixPeer,
            credential_kind: AgentSessionCredentialKind::LocalPeerBinding,
            max_ttl_ms: policy.max_ttl_ms,
            agent_types: policy.agent_types.clone(),
            capabilities: policy.capabilities.clone(),
        });
    }
    let mut authority = AgentSessionAuthority {
        schema: AGENT_SESSION_AUTHORITY_SCHEMA.to_string(),
        revision,
        authority_digest: String::new(),
        peer_observation_max_age_ms,
        enrollments,
    };
    authority.authority_digest = agent_session_authority_digest(&authority)
        .map_err(|error| format!("agent enrollment authority digest failed: {error}"))?;
    Ok(authority)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_uses_observed_uid_and_content_address() {
        let authority = compose_agent_session_authority(
            "node-a",
            1,
            2_000,
            &[AgentEnrollmentPolicy {
                observed_linux_uid: 1000,
                agent_types: vec!["codex".to_string()],
                max_ttl_ms: 900_000,
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
                .any(|grant| grant.id == "runtime.read" && !grant.mutates)
        );
        assert!(
            capabilities
                .iter()
                .any(|grant| grant.id == "agent_workload.delivery" && grant.mutates)
        );
    }
}
