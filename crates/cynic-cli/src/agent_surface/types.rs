//! Pure read-back shapes for agent coordination surfaces.
//!
//! These types are projections of the canonical coordination protocol
//! defined in `cynic_protocol::agent_coordination`. They carry no I/O
//! logic — only serialization boundaries for CLI/MCP reports.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const AGENT_CLAIM_REPORT_SCHEMA: &str = "cynic.cli.agent_claim_report.v1";
pub(crate) const AGENT_RELEASE_REPORT_SCHEMA: &str = "cynic.cli.agent_release_report.v1";
pub(crate) const AGENT_STATUS_REPORT_SCHEMA: &str = "cynic.cli.agent_status_report.v1";
pub(crate) const AGENT_IDENTIFY_REPORT_SCHEMA: &str = "cynic.cli.agent_identify_report.v1";

/// Report emitted after a successful claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentClaimReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub agent_id: String,
    pub target: String,
    pub claim_type: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub ttl_seconds: u64,
    pub budget_actions: u64,
    pub budget_wall_seconds: u64,
    pub granted_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

/// Report emitted after a successful release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentReleaseReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub agent_id: String,
    pub target: String,
    pub lease_id: String,
    pub proof_refs: Vec<String>,
    pub released_at_unix_ms: u64,
}

/// Report emitted by status query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentStatusReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub agent_id: String,
    pub active_claims: Vec<AgentClaimSummary>,
    pub total_claims: usize,
}

/// Summary of a single active claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentClaimSummary {
    pub claim_id: String,
    pub target: String,
    pub claim_type: String,
    pub granted_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub remaining_seconds: i64,
}

/// Report emitted after identity declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentIdentifyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub agent_id: String,
    pub agent_kind: String,
    pub credential_kind: String,
    pub attested_at_unix_ms: u64,
}

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
