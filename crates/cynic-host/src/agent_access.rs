mod access_apply;
mod authority_apply;
mod authority_bundle;
mod authority_projection;
mod types;

// Domaine pur : L2 (cynic-agent). Ré-exports de compat pour les callers hôte.
pub use access_apply::{apply_agent_access, AgentAccessApplyReport};
pub use authority_apply::{apply_agent_authority_projection, AgentAuthorityApplyReport};
pub use authority_bundle::{
    write_agent_authority_bundle, AgentAuthorityBundleReport, AgentAuthorityBundleRequest,
};
pub use authority_projection::{
    installed_agent_authority_spec, observe_agent_authority_projection,
};
pub use cynic_agent::agent_access::{
    plan_agent_authority_projection, reconcile_agent_access, AgentAccessObservation,
    AgentAccessPlan, AgentAccessPolicy, AgentAuthorityProjectionObservation,
    AgentAuthorityProjectionPlan, AgentAuthorityProjectionSpec, AGENT_ACCESS_OBSERVATION_SCHEMA,
    AGENT_ACCESS_PLAN_SCHEMA, AGENT_ACCESS_POLICY_SCHEMA, AGENT_AUTHORITY_PROJECTION_SCHEMA,
};
pub use types::desired_agent_access_policy;
