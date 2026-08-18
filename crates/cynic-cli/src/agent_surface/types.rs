//! Read-back types for the HumanAdmin governance observer ("SOC-lite").

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A single declared role's authority + observed grant state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSocEntry {
    pub role: String,
    pub authority_class: String,
    pub secret_source_class: String,
    pub required_human_secret: bool,
    pub granted: bool,
    pub granted_via: String,
    pub can_mutate: Vec<String>,
    pub can_observe: Vec<String>,
    pub zero_touch_runtime: bool,
}

/// Aggregated SOC report reconciling declared roles (infra/agent-roles.json)
/// against observed runtime proof. HumanAdmin consumes this read-only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSocReport {
    pub schema: String,
    pub human_admin_role: HumanAdminStatus,
    pub roles: BTreeMap<String, AgentSocEntry>,
    pub domains: BTreeMap<String, DomainStatus>,
    /// Residual unknowns that a domain cannot currently observe.
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanAdminStatus {
    pub zero_touch_runtime: bool,
    pub never_touches_runtime: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStatus {
    pub human_admin_role: String,
    pub human_admin_action: String,
    pub agent_role: String,
    pub agent_action: String,
    pub human_approval_required: bool,
    pub mutation_policy: String,
    pub observed: bool,
}
