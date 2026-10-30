use serde::{Deserialize, Serialize};

use super::*;

// ── Agent Identity (Phase 1 — IDENTIFY) ──────────────────────────────────

/// What kind of agent is this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Hermes,
    Codex,
    Claude,
    CynicNative,
    HumanAdmin,
}

/// Attested credential binding — proves who the agent is without
/// revealing the credential itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CredentialAttestation {
    LocalPeerBinding {
        process_id: u32,
        user_id: u32,
        attested_at_unix_ms: u64,
    },
    OpaqueReference {
        reference: String,
        attested_at_unix_ms: u64,
    },
    SignedJwt {
        issuer: String,
        subject: String,
        expires_at_unix_ms: u64,
        attested_at_unix_ms: u64,
    },
}

/// Declared identity of an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub schema: String,
    pub agent_id: String,
    pub agent_kind: AgentKind,
    pub credentials: CredentialAttestation,
    pub attested_at_unix_ms: u64,
    pub evidence: Option<SecurityEvidenceState>,
}

impl AgentIdentity {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut codes = Vec::new();

        if self.schema != AGENT_IDENTITY_SCHEMA {
            codes.push("agent_identity_schema_invalid".to_string());
        }
        if self.agent_id.trim().is_empty() {
            codes.push("agent_id_empty".to_string());
        }
        if self.agent_id.chars().any(char::is_control) {
            codes.push("agent_id_control_characters".to_string());
        }
        if self.agent_id.chars().count() > 64 {
            codes.push("agent_id_too_long".to_string());
        }
        if self.attested_at_unix_ms == 0 {
            codes.push("attestation_timestamp_missing".to_string());
        }

        match &self.credentials {
            CredentialAttestation::LocalPeerBinding { user_id, .. } if *user_id == 0 => {
                codes.push("local_peer_root_user_forbidden".to_string());
            }
            CredentialAttestation::SignedJwt {
                expires_at_unix_ms, ..
            } if *expires_at_unix_ms <= self.attested_at_unix_ms => {
                codes.push("jwt_expiry_before_attestation".to_string());
            }
            _ => {}
        }

        if codes.is_empty() { Ok(()) } else { Err(codes) }
    }
}

// ── Registration (Phase 2 — REGISTER) ────────────────────────────────────

/// Record of a successful agent registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationRecord {
    pub schema: String,
    pub agent_id: String,
    pub session_id: String,
    pub registered_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub capabilities: Vec<String>,
}

impl RegistrationRecord {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut codes = Vec::new();

        if self.schema != REGISTRATION_RECORD_SCHEMA {
            codes.push("registration_schema_invalid".to_string());
        }
        if self.agent_id.trim().is_empty() {
            codes.push("agent_id_empty".to_string());
        }
        if self.session_id.trim().is_empty() {
            codes.push("session_id_empty".to_string());
        }
        if self.expires_at_unix_ms <= self.registered_at_unix_ms {
            codes.push("registration_lifetime_invalid".to_string());
        }

        if codes.is_empty() { Ok(()) } else { Err(codes) }
    }
}

