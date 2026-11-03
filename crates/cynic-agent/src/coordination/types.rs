//! Domain types for agent coordination (pure, no I/O).

use serde::{Deserialize, Serialize};

pub const AGENT_STATUS_REPORT_SCHEMA: &str = "cynic.cli.agent_status_report.v1";
pub const AGENT_CLAIM_REPORT_SCHEMA: &str = "cynic.cli.agent_claim_report.v1";
pub const AGENT_RELEASE_REPORT_SCHEMA: &str = "cynic.cli.agent_release_report.v1";
pub const AGENT_IDENTIFY_REPORT_SCHEMA: &str = "cynic.cli.agent_identify_report.v1";
pub const AGENT_EVENTS_REPORT_SCHEMA: &str = "cynic.cli.agent_events_report.v1";
pub const AGENT_SCOPE_REPORT_SCHEMA: &str = "cynic.agent.scope_report.v1";

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

/// Report emitted when reading back the append-only claim event log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventsReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub agent_id: String,
    pub target: String,
    pub total_events: usize,
    pub events: Vec<serde_json::Value>,
}

/// Report emitted for agent scope verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentScopeReport {
    pub schema: &'static str,
    pub agent_id: String,
    pub target_paths: Vec<String>,
    pub fully_claimed: bool,
    pub missing_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentClaimSummary {
    pub claim_id: String,
    pub target: String,
    pub claim_type: String,
    pub granted_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub remaining_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub agent_id: String,
    pub active_claims: Vec<AgentClaimSummary>,
    pub total_claims: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimEvent {
    pub schema: &'static str,
    pub event: String,
    pub agent_id: String,
    pub target: String,
    pub timestamp_unix_s: u64,
    pub detail: serde_json::Value,
}

pub type LocalScope = cynic_protocol::agent_coordination::ClaimScope;
