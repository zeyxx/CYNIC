use serde::Serialize;

use crate::ports::HostOperation;

pub const AGENT_ACCESS_POLICY_SCHEMA: &str = "cynic.host.agent_access_policy.v1";
pub const AGENT_ACCESS_OBSERVATION_SCHEMA: &str = "cynic.host.agent_access_observation.v1";
pub const AGENT_ACCESS_PLAN_SCHEMA: &str = "cynic.host.agent_access_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAccessPolicy {
    pub schema: String,
    pub runtime_user: String,
    pub access_group: String,
    pub authorized_principals: Vec<String>,
    pub runtime_directory: String,
    pub socket_path: String,
    pub socket_mode: u32,
}

impl AgentAccessPolicy {
    pub fn desired(authorized_principals: Vec<String>) -> Self {
        let authority = crate::runtime_authority::default_runtime_authority_paths();
        let runtime_directory = authority
            .kernel_socket_path
            .parent()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        Self {
            schema: AGENT_ACCESS_POLICY_SCHEMA.to_string(),
            runtime_user: authority.principal.runtime_user.clone(),
            access_group: authority.agent_access_group.clone(),
            authorized_principals,
            runtime_directory,
            socket_path: authority.kernel_socket_path.display().to_string(),
            socket_mode: 0o660,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAccessObservation {
    pub schema: String,
    pub access_group_present: Option<bool>,
    pub access_group_id: Option<u32>,
    pub observed_members: Vec<String>,
    pub runtime_directory_present: Option<bool>,
    pub socket_present: Option<bool>,
    pub socket_group_id: Option<u32>,
    pub socket_mode: Option<u32>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAccessPlan {
    pub schema: String,
    pub status: String,
    pub operations: Vec<HostOperation>,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
    pub expected_proofs: Vec<String>,
}
