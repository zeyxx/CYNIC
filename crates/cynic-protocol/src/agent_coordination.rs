//! Agent coordination protocol — canonical types and pure predicates.
//!
//! This module extends the coordination primitives from `coordination_lease`
//! with the full lifecycle: identity, claim, heartbeat, release, mutation.
//!
//! Existing `LeaseGuard`/`LeaseGrant` types are reused for backward
//! compatibility. New types (`Claim`, `ClaimRequest`, etc.) build on them.

pub mod predicates;

use serde::{Deserialize, Serialize};

// ── Re-exports from existing coordination_lease (backward compat) ────────

pub use crate::coordination_lease::{
    BatchClaimResult, LeaseAction, LeaseConflict, LeaseGrant, LeaseGuard, LeaseMutationOutcome,
    LeaseRejectionCode, TargetClaimOutcome,
};

use crate::error_catalog::CynicErrorCode;

pub use crate::agent_citizenship::{AgentBudgets, MutationAdmission};
pub use crate::agent_session::types::{
    AgentRootLaunchRequest, AgentSessionAuthority, AgentSessionCredentialKind,
    AgentSessionEnrollment, AgentSessionPeerObservation, AgentSessionRecord,
    AgentSessionRenewalRequest, AgentSessionRequest, AgentSessionState, AgentSessionTransport,
};

pub use crate::SecurityEvidenceState;

// ── Protocol version ──────────────────────────────────────────────────────

pub const AGENT_COORDINATION_SCHEMA: &str = "cynic.agent_coordination.v1";
pub const AGENT_IDENTITY_SCHEMA: &str = "cynic.agent_identity.v1";
pub const CLAIM_REQUEST_SCHEMA: &str = "cynic.claim_request.v1";
pub const CLAIM_SCHEMA: &str = "cynic.claim.v1";
pub const HEARTBEAT_REQUEST_SCHEMA: &str = "cynic.heartbeat_request.v1";
pub const RELEASE_REQUEST_SCHEMA: &str = "cynic.release_request.v1";
pub const REGISTRATION_RECORD_SCHEMA: &str = "cynic.registration_record.v1";

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

// ── Claim (Phase 3 — CLAIM) ──────────────────────────────────────────────

/// Hierarchical claim scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClaimScope {
    Fleet,
    Node {
        node_id: String,
    },
    Crate {
        crate_name: String,
    },
    Module {
        crate_name: String,
        module_path: String,
    },
    File {
        path: String,
    },
}

impl ClaimScope {
    pub fn specificity(&self) -> u8 {
        match self {
            Self::Fleet => 0,
            Self::Node { .. } => 1,
            Self::Crate { .. } => 2,
            Self::Module { .. } => 3,
            Self::File { .. } => 4,
        }
    }

    pub fn contains(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Fleet, _) => true,
            (Self::Node { node_id: a }, Self::Node { node_id: b }) => a == b,
            (Self::Node { .. }, _) => true,
            (Self::Crate { crate_name: a }, Self::Crate { crate_name: b }) => a == b,
            (Self::Crate { crate_name: a }, Self::Module { crate_name: b, .. }) => a == b,
            (Self::Crate { crate_name: a }, Self::File { path: b }) => {
                b.starts_with(a) || b.contains(&format!("{}/", a)) || b.contains(&format!("{}-", a))
            }
            (
                Self::Module {
                    crate_name: a,
                    module_path: b,
                },
                Self::Module {
                    crate_name: c,
                    module_path: d,
                },
            ) => a == c && b == d,
            (
                Self::Module {
                    crate_name: a,
                    module_path: b,
                },
                Self::File { path: c },
            ) => {
                c.starts_with(&format!("{}/{}/", a, b))
                    || c.starts_with(&format!("{}/src/{}/", a, b))
            }
            (Self::File { path: a }, Self::File { path: b }) => a == b,
            _ => false,
        }
    }
}

/// Type of claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    Exclusive,
    SharedObserve,
}

/// Status of a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Active,
    Expired,
    Released,
    Preempted,
    Blocked,
}

/// Thermodynamic phase of an agent — RFC-001 §3.4.
///
/// An agent in `Exploration` phase may read broadly but must not mutate the
/// worktree. Transitioning to `Crystallization` locks the agent's spin along
/// its orientation field and activates hard topological exclusion (INV-05).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentPhase {
    /// High-T: broad read-only observation, repulsion deactivated.
    #[default]
    Exploration,
    /// Zero-T: spin locked on intent vector, hard exclusion enforced.
    Crystallization,
}

impl AgentPhase {
    /// Serde default helper — new claims start in Exploration.
    pub fn default_exploration() -> Self {
        Self::Exploration
    }
}

/// Request to claim a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimRequest {
    pub schema: String,
    pub agent_id: String,
    pub session_id: String,
    pub scope: ClaimScope,
    pub claim_type: ClaimType,
    pub budget: AgentBudgets,
    pub ttl_seconds: u64,
    pub max_lease_window_ms: u64,
    pub parent_claim_id: Option<String>,
}

impl ClaimRequest {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut codes = Vec::new();

        if self.schema != CLAIM_REQUEST_SCHEMA {
            codes.push("claim_request_schema_invalid".to_string());
        }
        if self.agent_id.trim().is_empty() {
            codes.push("agent_id_empty".to_string());
        }
        if self.session_id.trim().is_empty() {
            codes.push("session_id_empty".to_string());
        }
        if self.ttl_seconds == 0 {
            codes.push("ttl_seconds_zero".to_string());
        }

        if codes.is_empty() { Ok(()) } else { Err(codes) }
    }
}

/// A granted claim with lease and fencing token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub schema: String,
    pub claim_id: String,
    pub request: ClaimRequest,
    pub lease_id: String,
    pub fencing_token: u64,
    pub granted_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub max_lease_window_ms: u64,
    pub status: ClaimStatus,
    pub preempted_by: Option<String>,
    /// Thermodynamic phase — RFC-001 §3.4. Defaults to Exploration.
    #[serde(default = "AgentPhase::default_exploration")]
    pub phase: AgentPhase,
    /// Budget actually consumed so far. Must never exceed `request.budget`
    /// (monotonic, saturating). Mirrors `consumed` in
    /// `cynic.agent_claims.v2`.
    #[serde(default)]
    pub consumed: AgentBudgets,
}

impl Claim {
    /// Remaining budget = granted budget − consumed budget (saturating).
    pub fn remaining_budget(&self) -> AgentBudgets {
        AgentBudgets {
            actions: self
                .request
                .budget
                .actions
                .saturating_sub(self.consumed.actions),
            wall_seconds: self
                .request
                .budget
                .wall_seconds
                .saturating_sub(self.consumed.wall_seconds),
        }
    }

    /// Pure consumption step: returns a new claim with the delta added to
    /// `consumed`, saturating at the granted budget. Refuses nothing here —
    /// admission is decided by [`admit_mutation`]; this only records.
    pub fn record_consumption(&self, delta: &AgentBudgets) -> Claim {
        let mut next = self.clone();
        next.consumed.actions = next.consumed.actions.saturating_add(delta.actions);
        next.consumed.wall_seconds = next
            .consumed
            .wall_seconds
            .saturating_add(delta.wall_seconds);
        if next.consumed.actions > next.request.budget.actions {
            next.consumed.actions = next.request.budget.actions;
        }
        if next.consumed.wall_seconds > next.request.budget.wall_seconds {
            next.consumed.wall_seconds = next.request.budget.wall_seconds;
        }
        next
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, ClaimStatus::Active)
    }

    pub fn is_expired_at(&self, now_unix_ms: u64) -> bool {
        now_unix_ms >= self.expires_at_unix_ms
    }

    pub fn is_heartbeat_stale(&self, now_unix_ms: u64) -> bool {
        let ttl_ms = self
            .expires_at_unix_ms
            .saturating_sub(self.granted_at_unix_ms);
        let stale_threshold = self.granted_at_unix_ms.saturating_add(ttl_ms * 8 / 10);
        now_unix_ms >= stale_threshold
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

// ── Mutation Admission (Phase 5 — MUTATE) ────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationRequest {
    pub schema: String,
    pub agent_id: String,
    pub claim_id: String,
    pub target_scope: ClaimScope,
    pub action: MutationAction,
    pub budget_delta: AgentBudgets,
    pub rollback_intent: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationAction {
    pub argv: Vec<String>,
    pub working_directory: Option<String>,
    pub mutates: bool,
    pub requires_privilege: bool,
    pub stop_condition: String,
}

impl MutationRequest {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut codes = Vec::new();

        if self.target_scope.specificity() == 0 {
            codes.push("mutation_target_too_broad".to_string());
        }
        if self.budget_delta.actions == 0 && self.budget_delta.wall_seconds == 0 {
            codes.push("budget_delta_zero".to_string());
        }

        if codes.is_empty() { Ok(()) } else { Err(codes) }
    }
}

/// Admit or refuse a mutation request against the active claim.
pub fn admit_mutation(
    claim: &Claim,
    request: &MutationRequest,
    now_unix_ms: u64,
    store_available: bool,
) -> MutationAdmission {
    let mut codes = Vec::new();

    if claim.status != ClaimStatus::Active {
        codes.push(CoordinationCode::ClaimNotActive);
    }

    if claim.is_expired_at(now_unix_ms) {
        codes.push(CoordinationCode::ClaimExpired);
    }

    if !claim.request.scope.contains(&request.target_scope) {
        codes.push(CoordinationCode::ScopeConflict);
    }

    let remaining = claim.remaining_budget();
    if request.budget_delta.actions > remaining.actions
        || request.budget_delta.wall_seconds > remaining.wall_seconds
    {
        codes.push(CoordinationCode::BudgetExceeded);
    }

    if !store_available {
        codes.push(CoordinationCode::StoreUnavailable);
    }

    if codes.is_empty() {
        MutationAdmission::Admitted
    } else {
        MutationAdmission::Refused {
            codes: codes.into_iter().map(CynicErrorCode::from).collect(),
        }
    }
}

// ── Coordination error codes ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofKind {
    Claim,
    Heartbeat,
    Release,
    Mutation,
    Conflict,
    Sync,
}

/// Envelope for any coordination proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofEnvelope {
    pub schema: String,
    pub proof_id: String,
    pub kind: ProofKind,
    pub subject_id: String,
    pub produced_at_unix_ms: u64,
    pub producer_id: String,
    pub evidence_refs: Vec<String>,
    pub digest: String,
}

/// Proof emitted after a successful claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimProof {
    pub schema: String,
    pub claim_id: String,
    pub lease_id: String,
    pub evidence_refs: Vec<String>,
    pub proven_at_unix_ms: u64,
}

/// Proof emitted after a successful release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseProof {
    pub schema: String,
    pub lease_id: String,
    pub released_at_unix_ms: u64,
    pub proof_refs: Vec<String>,
    pub final_state_digest: String,
}

/// Proof emitted after a successful heartbeat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatProof {
    pub schema: String,
    pub lease_id: String,
    pub previous_expiry_ms: u64,
    pub new_expiry_ms: u64,
    pub beats_at_unix_ms: u64,
}

/// Proof emitted after each bounded mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationProof {
    pub schema: String,
    pub claim_id: String,
    pub mutation_id: String,
    pub target_scope: ClaimScope,
    pub argv: Vec<String>,
    pub exit_code: i32,
    pub evidence_refs: Vec<String>,
    pub mutated_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationCode {
    ClaimNotActive,
    ClaimExpired,
    ScopeConflict,
    BudgetExceeded,
    StoreUnavailable,
    LeaseIdMismatch,
    FencingTokenMismatch,
}

impl From<CoordinationCode> for CynicErrorCode {
    fn from(code: CoordinationCode) -> Self {
        match code {
            CoordinationCode::ClaimNotActive => CynicErrorCode::AgentMutationWithoutClaim,
            CoordinationCode::ClaimExpired => CynicErrorCode::AgentClaimExpired,
            CoordinationCode::ScopeConflict => CynicErrorCode::AgentScopeConflict,
            CoordinationCode::BudgetExceeded => CynicErrorCode::AgentBudgetExceeded,
            CoordinationCode::StoreUnavailable => CynicErrorCode::AgentCoordinationStoreUnavailable,
            CoordinationCode::LeaseIdMismatch => CynicErrorCode::AgentMutationWithoutClaim,
            CoordinationCode::FencingTokenMismatch => CynicErrorCode::AgentMutationWithoutClaim,
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_agent_identity_passes_validation() {
        let identity = AgentIdentity {
            schema: AGENT_IDENTITY_SCHEMA.to_string(),
            agent_id: "hermes:test:node1:1234".to_string(),
            agent_kind: AgentKind::Hermes,
            credentials: CredentialAttestation::LocalPeerBinding {
                process_id: 1234,
                user_id: 1000,
                attested_at_unix_ms: 1000,
            },
            attested_at_unix_ms: 1000,
            evidence: Some(SecurityEvidenceState::Proven),
        };
        assert!(identity.validate().is_ok());
    }

    #[test]
    fn empty_agent_id_fails_validation() {
        let identity = AgentIdentity {
            schema: AGENT_IDENTITY_SCHEMA.to_string(),
            agent_id: "".to_string(),
            agent_kind: AgentKind::Hermes,
            credentials: CredentialAttestation::LocalPeerBinding {
                process_id: 1234,
                user_id: 1000,
                attested_at_unix_ms: 1000,
            },
            attested_at_unix_ms: 1000,
            evidence: None,
        };
        assert!(identity.validate().is_err());
    }

    #[test]
    fn jwt_expiry_before_attestation_fails() {
        let identity = AgentIdentity {
            schema: AGENT_IDENTITY_SCHEMA.to_string(),
            agent_id: "hermes:test".to_string(),
            agent_kind: AgentKind::Hermes,
            credentials: CredentialAttestation::SignedJwt {
                issuer: "test".to_string(),
                subject: "hermes".to_string(),
                expires_at_unix_ms: 100,
                attested_at_unix_ms: 1000,
            },
            attested_at_unix_ms: 1000,
            evidence: None,
        };
        assert!(identity.validate().is_err());
    }

    #[test]
    fn fleet_scope_contains_everything() {
        let fleet = ClaimScope::Fleet;
        assert!(fleet.contains(&ClaimScope::Node {
            node_id: "node1".to_string()
        }));
        assert!(fleet.contains(&ClaimScope::Crate {
            crate_name: "cynic-protocol".to_string()
        }));
        assert!(fleet.contains(&ClaimScope::File {
            path: "crates/cynic-protocol/src/lib.rs".to_string()
        }));
    }

    #[test]
    fn crate_scope_contains_module_and_file() {
        let crate_scope = ClaimScope::Crate {
            crate_name: "cynic-protocol".to_string(),
        };
        assert!(crate_scope.contains(&ClaimScope::Module {
            crate_name: "cynic-protocol".to_string(),
            module_path: "src".to_string(),
        }));
        assert!(crate_scope.contains(&ClaimScope::File {
            path: "crates/cynic-protocol/src/lib.rs".to_string(),
        }));
    }

    #[test]
    fn active_claim_is_active() {
        let claim = Claim {
            schema: CLAIM_SCHEMA.to_string(),
            claim_id: "claim_001".to_string(),
            request: claim_request(
                "hermes".to_string(),
                ClaimScope::Crate {
                    crate_name: "cynic-protocol".to_string(),
                },
            ),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 10000,
            status: ClaimStatus::Active,
            preempted_by: None,
            phase: AgentPhase::Exploration,
            consumed: AgentBudgets::default(),
        };
        assert!(claim.is_active());
        assert!(!claim.is_expired_at(4000));
        assert!(claim.is_expired_at(6000));
    }

    #[test]
    fn valid_heartbeat_passes() {
        let claim = Claim {
            schema: CLAIM_SCHEMA.to_string(),
            claim_id: "claim_001".to_string(),
            request: claim_request(
                "hermes".to_string(),
                ClaimScope::Crate {
                    crate_name: "cynic-protocol".to_string(),
                },
            ),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 10000,
            status: ClaimStatus::Active,
            preempted_by: None,
            phase: AgentPhase::Exploration,
            consumed: AgentBudgets::default(),
        };
        let heartbeat = HeartbeatRequest {
            schema: HEARTBEAT_REQUEST_SCHEMA.to_string(),
            agent_id: "hermes".to_string(),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
        };
        assert!(validate_heartbeat(&heartbeat, &claim, 3000).is_ok());
    }

    #[test]
    fn heartbeat_with_wrong_fencing_token_fails() {
        let claim = Claim {
            schema: CLAIM_SCHEMA.to_string(),
            claim_id: "claim_001".to_string(),
            request: claim_request(
                "hermes".to_string(),
                ClaimScope::Crate {
                    crate_name: "cynic-protocol".to_string(),
                },
            ),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 10000,
            status: ClaimStatus::Active,
            preempted_by: None,
            phase: AgentPhase::Exploration,
            consumed: AgentBudgets::default(),
        };
        let heartbeat = HeartbeatRequest {
            schema: HEARTBEAT_REQUEST_SCHEMA.to_string(),
            agent_id: "hermes".to_string(),
            lease_id: "lease_001".to_string(),
            fencing_token: 2,
        };
        assert!(validate_heartbeat(&heartbeat, &claim, 3000).is_err());
    }

    #[test]
    fn admit_mutation_within_scope() {
        let claim = make_claim_with_consumed(AgentBudgets::default());
        let request = mutation_request(
            ClaimScope::File {
                path: "crates/cynic-protocol/src/lib.rs".to_string(),
            },
            AgentBudgets {
                actions: 1,
                wall_seconds: 60,
            },
        );
        assert!(admit_mutation(&claim, &request, 3000, true).is_admitted());
    }

    #[test]
    fn mutation_outside_scope_is_refused() {
        let claim = Claim {
            schema: CLAIM_SCHEMA.to_string(),
            claim_id: "claim_001".to_string(),
            request: claim_request(
                "hermes".to_string(),
                ClaimScope::Crate {
                    crate_name: "cynic-protocol".to_string(),
                },
            ),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 10000,
            status: ClaimStatus::Active,
            preempted_by: None,
            phase: AgentPhase::Exploration,
            consumed: AgentBudgets::default(),
        };
        let request = MutationRequest {
            schema: "cynic.mutation_request.v1".to_string(),
            agent_id: "hermes".to_string(),
            claim_id: "claim_001".to_string(),
            target_scope: ClaimScope::Crate {
                crate_name: "cynic-kernel".to_string(),
            },
            action: MutationAction {
                argv: vec!["make".to_string(), "check".to_string()],
                working_directory: None,
                mutates: true,
                requires_privilege: false,
                stop_condition: "stop on first failure".to_string(),
            },
            budget_delta: AgentBudgets {
                actions: 1,
                wall_seconds: 60,
            },
            rollback_intent: "discard changes".to_string(),
        };
        assert!(!admit_mutation(&claim, &request, 3000, true).is_admitted());
    }

    #[test]
    fn consumed_budget_reduces_remaining() {
        let claim = make_claim_with_consumed(AgentBudgets {
            actions: 49,
            wall_seconds: 3540,
        });
        let request = mutation_request(
            ClaimScope::File {
                path: "crates/cynic-protocol/src/lib.rs".to_string(),
            },
            AgentBudgets {
                actions: 2,
                wall_seconds: 60,
            },
        );
        assert!(matches!(
            admit_mutation(&claim, &request, 3000, true),
            MutationAdmission::Refused { .. }
        ));
    }

    #[test]
    fn record_consumption_is_monotonic_and_saturating() {
        let claim = make_claim_with_consumed(AgentBudgets::default());
        let next = claim.record_consumption(&AgentBudgets {
            actions: 40,
            wall_seconds: 1800,
        });
        assert_eq!(next.consumed.actions, 40);
        assert_eq!(next.consumed.wall_seconds, 1800);
        assert_eq!(next.remaining_budget().actions, 10);

        let saturated = next.record_consumption(&AgentBudgets {
            actions: u64::MAX,
            wall_seconds: u64::MAX,
        });
        assert_eq!(saturated.consumed, saturated.request.budget);
        assert_eq!(saturated.remaining_budget().actions, 0);
        assert_eq!(saturated.remaining_budget().wall_seconds, 0);
    }

    #[test]
    fn exhausted_budget_refuses_mutation() {
        let claim = make_claim_with_consumed(AgentBudgets {
            actions: 50,
            wall_seconds: 3600,
        });
        let request = mutation_request(
            ClaimScope::File {
                path: "crates/cynic-protocol/src/lib.rs".to_string(),
            },
            AgentBudgets {
                actions: 1,
                wall_seconds: 60,
            },
        );
        assert!(!admit_mutation(&claim, &request, 3000, true).is_admitted());
    }

    fn make_claim_with_consumed(consumed: AgentBudgets) -> Claim {
        Claim {
            schema: CLAIM_SCHEMA.to_string(),
            claim_id: "claim_001".to_string(),
            request: claim_request(
                "hermes".to_string(),
                ClaimScope::Crate {
                    crate_name: "cynic-protocol".to_string(),
                },
            ),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 10000,
            status: ClaimStatus::Active,
            preempted_by: None,
            phase: AgentPhase::Exploration,
            consumed,
        }
    }

    fn mutation_request(target_scope: ClaimScope, budget_delta: AgentBudgets) -> MutationRequest {
        MutationRequest {
            schema: "cynic.mutation_request.v1".to_string(),
            agent_id: "hermes".to_string(),
            claim_id: "claim_001".to_string(),
            target_scope,
            action: MutationAction {
                argv: vec!["make".to_string(), "check".to_string()],
                working_directory: None,
                mutates: true,
                requires_privilege: false,
                stop_condition: "stop on first failure".to_string(),
            },
            budget_delta,
            rollback_intent: "discard changes".to_string(),
        }
    }

    fn claim_request(agent_id: String, scope: ClaimScope) -> ClaimRequest {
        ClaimRequest {
            schema: CLAIM_REQUEST_SCHEMA.to_string(),
            agent_id,
            session_id: "session_001".to_string(),
            scope,
            claim_type: ClaimType::Exclusive,
            budget: AgentBudgets {
                actions: 50,
                wall_seconds: 3600,
            },
            ttl_seconds: 1800,
            max_lease_window_ms: 3600000,
            parent_claim_id: None,
        }
    }
}

// ── RFC-001: AgentFieldEnvelope — Parent induction field ──────────────────

/// The orientation field emitted by a parent agent to a child.
///
/// A child agent is spawned with ONLY this envelope — never with raw chat
/// history. It encodes the 4-tuple from RFC-001 §3.3:
///   B = <intent_vector, invariant_constraints, output_schema, budget>.
///
/// The child aligns its execution along this field without requiring any
/// knowledge of the parent's conversational context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentFieldEnvelope {
    pub schema: String,
    /// Unique identifier for this delegation envelope.
    pub envelope_id: String,
    /// Parent agent emitting the field.
    pub parent_agent_id: String,
    /// Parent claim authorizing this delegation (monotonic budget split).
    pub parent_claim_id: String,
    /// Recursion depth: child = parent_depth + 1. Never decrements.
    pub recursion_depth: u32,
    /// Compact, unambiguous intent (the direction of B). Max 512 chars.
    pub intent_vector: String,
    /// Hard boolean constraints the child must never violate.
    pub invariant_constraints: Vec<String>,
    /// JSON schema name the child's completion payload must conform to.
    pub output_schema: String,
    /// Strictly clamped budget — must be <= parent's remaining budget (INV-01).
    pub budget: AgentBudgets,
    /// Scopes the child is permitted to touch (empty = inherits parent scope).
    pub permitted_scopes: Vec<ClaimScope>,
    /// Scopes explicitly forbidden to the child (topological exclusion zones).
    pub forbidden_scopes: Vec<ClaimScope>,
    /// Wall-clock deadline (unix ms). Child must exit before this timestamp.
    pub deadline_unix_ms: u64,
}

pub const AGENT_FIELD_ENVELOPE_SCHEMA: &str = "cynic.agent_field_envelope.v1";

impl AgentFieldEnvelope {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut codes = Vec::new();
        if self.schema != AGENT_FIELD_ENVELOPE_SCHEMA {
            codes.push("field_envelope_schema_invalid".to_string());
        }
        if self.envelope_id.trim().is_empty() {
            codes.push("envelope_id_empty".to_string());
        }
        if self.parent_agent_id.trim().is_empty() {
            codes.push("parent_agent_id_empty".to_string());
        }
        if self.intent_vector.trim().is_empty() {
            codes.push("intent_vector_empty".to_string());
        }
        if self.intent_vector.chars().count() > 512 {
            codes.push("intent_vector_too_long".to_string());
        }
        if self.budget.actions == 0 {
            codes.push("budget_actions_zero".to_string());
        }
        if self.deadline_unix_ms == 0 {
            codes.push("deadline_missing".to_string());
        }
        if codes.is_empty() { Ok(()) } else { Err(codes) }
    }

    /// Verifies the envelope against the parent claim authorizing this delegation.
    /// Enforces RFC-001 INV-01: child budget <= parent remaining budget,
    /// and parent_claim_id matches.
    pub fn validate_against_parent(&self, parent_claim: &Claim) -> Result<(), Vec<String>> {
        let mut codes = self.validate().err().unwrap_or_default();
        if parent_claim.claim_id != self.parent_claim_id {
            codes.push("field_envelope_parent_claim_mismatch".to_string());
        }
        let remaining = parent_claim.remaining_budget();
        if self.budget.actions > remaining.actions {
            codes.push("child_budget_actions_exceeds_parent".to_string());
        }
        if self.budget.wall_seconds > remaining.wall_seconds {
            codes.push("child_budget_wall_seconds_exceeds_parent".to_string());
        }
        if codes.is_empty() { Ok(()) } else { Err(codes) }
    }
}
