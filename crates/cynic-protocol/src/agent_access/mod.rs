//! Domaine pur `agent_access` (L0) — Verticale 4/4.
//!
//! Déplacé vers L0 depuis `cynic-agent` (L2), lui-même extrait de `cynic-host/src/agent_access/{decision,types}.rs` et
//! `authority_projection.rs` (partie pure). Purifié :
//! - `operations: Vec<HostOperation>` -> `Vec<AgentAccessOperation>` (port L0,
//!   `cynic_protocol::agent_access_port`) ;
//! - `AgentAccessPolicy::desired()` (lisait `cynic-registry`) supprimé :
//!   la construction registry-dépendante vit côté hôte
//!   (`cynic_host::agent_access::desired_agent_access_policy`) ;
//! - `AgentAuthorityProjectionSpec::installed()` (registry + `/etc/passwd`)
//!   et `observe_agent_authority_projection` (I/O FS) supprimés :
//!   côté hôte (`installed_agent_authority_spec`, observe conservé).
//!
//! Invariants conservés : `socket_mode == 0o660`, blocage sur mismatch
//! groupe/mode du socket ; projection : mode `0o640`, owner `root`,
//! source prouvée (digest), convergence stricte.

mod authority_projection;
mod decision;
mod types;

pub use authority_projection::{
    AGENT_AUTHORITY_PROJECTION_SCHEMA, AgentAuthorityProjectionObservation,
    AgentAuthorityProjectionPlan, AgentAuthorityProjectionSpec, plan_agent_authority_projection,
};
pub use decision::reconcile_agent_access;
pub use types::{
    AGENT_ACCESS_OBSERVATION_SCHEMA, AGENT_ACCESS_PLAN_SCHEMA, AGENT_ACCESS_POLICY_SCHEMA,
    AgentAccessObservation, AgentAccessPlan, AgentAccessPolicy,
};
