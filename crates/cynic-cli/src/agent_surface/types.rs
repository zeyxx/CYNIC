//! Pure read-back shapes for agent coordination surfaces.
//!
//! These types are projections of the canonical coordination protocol
//! defined in `cynic_protocol::agent_coordination`. They carry no I/O
//! logic — only serialization boundaries for CLI/MCP reports.

use serde::{Deserialize, Serialize};

pub const AGENT_CLAIM_REPORT_SCHEMA: &str = "cynic.cli.agent_claim_report.v1";
pub const AGENT_RELEASE_REPORT_SCHEMA: &str = "cynic.cli.agent_release_report.v1";
pub const AGENT_STATUS_REPORT_SCHEMA: &str = "cynic.cli.agent_status_report.v1";
pub const AGENT_IDENTIFY_REPORT_SCHEMA: &str = "cynic.cli.agent_identify_report.v1";

/// Report emitted after a successful claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentClaimReport {
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
pub struct AgentReleaseReport {
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
pub struct AgentStatusReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub agent_id: String,
    pub active_claims: Vec<AgentClaimSummary>,
    pub total_claims: usize,
}

/// Summary of a single active claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentClaimSummary {
    pub claim_id: String,
    pub target: String,
    pub claim_type: String,
    pub granted_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub remaining_seconds: i64,
}

/// Report emitted after identity declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentifyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub agent_id: String,
    pub agent_kind: String,
    pub credential_kind: String,
    pub attested_at_unix_ms: u64,
}
