use serde::{Deserialize, Serialize};

use super::*;

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

