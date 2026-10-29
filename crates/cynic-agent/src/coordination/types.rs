//! Domain types for agent coordination (pure, no I/O).

use serde::{Deserialize, Serialize};

pub const AGENT_STATUS_REPORT_SCHEMA: &str = "cynic.cli.agent_status_report.v1";

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
