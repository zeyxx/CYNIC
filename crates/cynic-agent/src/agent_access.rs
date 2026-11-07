//! Compat shim: the pure `agent_access` domain now lives in L0
//! (`cynic_protocol::agent_access`). This re-export keeps the historical
//! `cynic_agent::agent_access` path working for existing callers; new
//! code should import from `cynic_protocol::agent_access` directly so
//! L1 crates need not depend on this L2 crate to reach it.

pub use cynic_protocol::agent_access::{
    AGENT_ACCESS_OBSERVATION_SCHEMA, AGENT_ACCESS_PLAN_SCHEMA, AGENT_ACCESS_POLICY_SCHEMA,
    AGENT_AUTHORITY_PROJECTION_SCHEMA, AgentAccessObservation, AgentAccessPlan, AgentAccessPolicy,
    AgentAuthorityProjectionObservation, AgentAuthorityProjectionPlan,
    AgentAuthorityProjectionSpec, plan_agent_authority_projection, reconcile_agent_access,
};
