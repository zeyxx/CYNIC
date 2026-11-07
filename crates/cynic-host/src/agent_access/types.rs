//! Compat hôte : les types purs vivent en L0 (`cynic_protocol::agent_access`).
//! Ce module ne conserve que le constructeur dépendant du registry, que le
//! domaine pur ne peut pas connaître.

use cynic_protocol::agent_access::{AGENT_ACCESS_POLICY_SCHEMA, AgentAccessPolicy};

/// Construit la politique désirée à partir du registry (host-only).
pub fn desired_agent_access_policy(authorized_principals: Vec<String>) -> AgentAccessPolicy {
    let authority = cynic_registry::runtime_authority::default_runtime_authority_paths();
    let runtime_directory = authority
        .kernel_socket_path
        .parent()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    AgentAccessPolicy {
        schema: AGENT_ACCESS_POLICY_SCHEMA.to_string(),
        runtime_user: authority.principal.runtime_user.clone(),
        access_group: authority.agent_access_group.clone(),
        authorized_principals,
        runtime_directory,
        socket_path: authority.kernel_socket_path.display().to_string(),
        socket_mode: 0o660,
    }
}
