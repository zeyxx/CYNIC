mod access_apply;
mod authority;
mod authority_apply;
mod authority_bundle;
mod authority_projection;
mod decision;
mod types;

pub use access_apply::{AgentAccessApplyReport, apply_agent_access};
pub use authority::{
    AgentEnrollmentPolicy, DEFAULT_AGENT_SESSION_TTL_MS, DEFAULT_PEER_OBSERVATION_MAX_AGE_MS,
    compose_agent_session_authority, default_local_agent_capabilities,
};
pub use authority_apply::{AgentAuthorityApplyReport, apply_agent_authority_projection};
pub use authority_bundle::{
    AgentAuthorityBundleReport, AgentAuthorityBundleRequest, write_agent_authority_bundle,
};
pub use authority_projection::{
    AGENT_AUTHORITY_PROJECTION_SCHEMA, AgentAuthorityProjectionObservation,
    AgentAuthorityProjectionPlan, AgentAuthorityProjectionSpec, observe_agent_authority_projection,
    plan_agent_authority_projection,
};
pub use decision::reconcile_agent_access;
pub use types::{
    AGENT_ACCESS_OBSERVATION_SCHEMA, AGENT_ACCESS_PLAN_SCHEMA, AGENT_ACCESS_POLICY_SCHEMA,
    AgentAccessObservation, AgentAccessPlan, AgentAccessPolicy,
};
