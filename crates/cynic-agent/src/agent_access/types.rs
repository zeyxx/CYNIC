use cynic_protocol::agent_access_port::AgentAccessOperation;
use serde::Serialize;

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
    /// Opérations fermées du port L0 ; l'adapter hôte les traduit en
    /// `HostOperation` (voir `cynic_host::adapters::agent_access_operation`).
    pub operations: Vec<AgentAccessOperation>,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
    pub expected_proofs: Vec<String>,
}
