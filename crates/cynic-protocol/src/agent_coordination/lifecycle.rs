use serde::{Deserialize, Serialize};

use super::*;

/// Thermodynamic phase of an agent — RFC-001 §3.4.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentPhase {
    #[default]
    Exploration,
    Crystallization,
}

impl AgentPhase {
    pub fn default_exploration() -> Self {
        Self::Exploration
    }
}

// ── Heartbeat (Phase 4 — HEARTBEAT) ──────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub schema: String,
    pub agent_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
}

impl HeartbeatRequest {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut codes = Vec::new();

        if self.schema != HEARTBEAT_REQUEST_SCHEMA {
            codes.push("heartbeat_request_schema_invalid".to_string());
        }
        if self.agent_id.trim().is_empty() {
            codes.push("agent_id_empty".to_string());
        }
        if self.lease_id.trim().is_empty() {
            codes.push("lease_id_empty".to_string());
        }

        if codes.is_empty() { Ok(()) } else { Err(codes) }
    }
}

/// Validate a heartbeat against the active claim it renews.
pub fn validate_heartbeat(
    request: &HeartbeatRequest,
    active_claim: &Claim,
    now_unix_ms: u64,
) -> Result<(), Vec<String>> {
    let mut codes = Vec::new();

    if request.lease_id != active_claim.lease_id {
        codes.push("lease_id_mismatch".to_string());
    }
    if request.fencing_token != active_claim.fencing_token {
        codes.push("fencing_token_mismatch".to_string());
    }
    if active_claim.is_expired_at(now_unix_ms) {
        codes.push("claim_expired".to_string());
    }
    if active_claim.status != ClaimStatus::Active {
        codes.push("claim_not_active".to_string());
    }

    if codes.is_empty() { Ok(()) } else { Err(codes) }
}

// ── Release (Phase 6 — RELEASE) ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseRequest {
    pub schema: String,
    pub agent_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub proof_refs: Vec<String>,
}

impl ReleaseRequest {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut codes = Vec::new();

        if self.schema != RELEASE_REQUEST_SCHEMA {
            codes.push("release_request_schema_invalid".to_string());
        }
        if self.agent_id.trim().is_empty() {
            codes.push("agent_id_empty".to_string());
        }
        if self.lease_id.trim().is_empty() {
            codes.push("lease_id_empty".to_string());
        }

        if codes.is_empty() { Ok(()) } else { Err(codes) }
    }
}

