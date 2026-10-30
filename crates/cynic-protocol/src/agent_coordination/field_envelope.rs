use serde::{Deserialize, Serialize};

use super::*;

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
