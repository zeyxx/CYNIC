//! Pure read-back shapes for agent coordination surfaces.
//!
//! These types are projections of the canonical coordination protocol
//! defined in `cynic_protocol::agent_coordination`. They carry no I/O
//! logic — only serialization boundaries for CLI/MCP reports.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) use cynic_agent::coordination::{
    AgentClaimReport, AgentEventsReport, AgentIdentifyReport, AgentReleaseReport,
    AgentScopeReport, AGENT_CLAIM_REPORT_SCHEMA, AGENT_EVENTS_REPORT_SCHEMA,
    AGENT_IDENTIFY_REPORT_SCHEMA, AGENT_RELEASE_REPORT_SCHEMA, AGENT_SCOPE_REPORT_SCHEMA,
};

// ── Agent SoC Audit Types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSocReport {
    pub schema: String,
    pub human_admin_role: HumanAdminStatus,
    pub roles: BTreeMap<String, AgentSocEntry>,
    pub domains: BTreeMap<String, DomainStatus>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanAdminStatus {
    pub zero_touch_runtime: bool,
    pub never_touches_runtime: Vec<String>,
}

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
