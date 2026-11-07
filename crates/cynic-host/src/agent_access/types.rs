//! Compat hôte : les types purs vivent en L2 (`cynic_agent::agent_access`).
//! Ce module ne conserve que le constructeur dépendant du registry, que le
//! domaine pur ne peut pas connaître.

pub use cynic_agent::agent_access::{
    AgentAccessObservation, AgentAccessPlan, AgentAccessPolicy, AGENT_ACCESS_OBSERVATION_SCHEMA,
    AGENT_ACCESS_PLAN_SCHEMA, AGENT_ACCESS_POLICY_SCHEMA,
};

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
