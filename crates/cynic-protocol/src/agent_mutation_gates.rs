//! Extended admission gates — domain-specific projections over
//! the canonical `admit_mutation`.
//!
//! Per AGENTS.md invariant #6: surfaces project canonical decisions;
//! they do not recompute policy. Each function in this module calls
//! `admit_mutation` first, then adds domain-specific checks.

use crate::agent_citizenship::admit_mutation;
use crate::error_catalog::CynicErrorCode;
use crate::AgentBudgets;
use crate::AgentCitizenshipRecord;
use crate::AgentScope;
use crate::MutationAdmission;

/// Build admission gate.
///
/// Projects the canonical admission gate for build surfaces
/// (build scheduler, webhook forgejo, CLI submit).
///
/// Performs the canonical checks (scope, budget, claim) then adds:
/// - Build role validation (the claimed role must match the requested role)
pub fn admit_build_mutation(
    record: &AgentCitizenshipRecord,
    held_claim: Option<&AgentScope>,
    requested: &AgentBudgets,
    target_scope: &AgentScope,
    store_available: bool,
    required_role: &str,
    requested_role: &str,
) -> MutationAdmission {
    let admission = admit_mutation(record, held_claim, requested, target_scope, store_available);
    if !admission.is_admitted() {
        return admission;
    }

    let mut extra_codes = Vec::new();

    if required_role != requested_role {
        extra_codes.push(CynicErrorCode::BuildAdmissionAuthorityMaterialMissing);
    }

    if !extra_codes.is_empty() {
        return MutationAdmission::Refused { codes: extra_codes };
    }

    MutationAdmission::Admitted
}

/// Workload admission gate.
///
/// Projects the canonical admission gate for workload surfaces.
///
/// Performs the canonical checks then adds:
/// - Workload claim expiry check (the lease must still be valid)
pub fn admit_workload_mutation(
    record: &AgentCitizenshipRecord,
    held_claim: Option<&AgentScope>,
    requested: &AgentBudgets,
    target_scope: &AgentScope,
    store_available: bool,
    workload_claim_expires_at_ms: Option<u64>,
    now_ms: u64,
) -> MutationAdmission {
    let admission = admit_mutation(record, held_claim, requested, target_scope, store_available);
    if !admission.is_admitted() {
        return admission;
    }

    let mut extra_codes = Vec::new();

    if let Some(expires_at_ms) = workload_claim_expires_at_ms {
        if expires_at_ms <= now_ms {
            extra_codes.push(CynicErrorCode::AgentClaimExpired);
        }
    } else {
        extra_codes.push(CynicErrorCode::AgentMutationWithoutClaim);
    }

    if !extra_codes.is_empty() {
        return MutationAdmission::Refused { codes: extra_codes };
    }

    MutationAdmission::Admitted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(actions: u64, wall_seconds: u64) -> AgentCitizenshipRecord {
        AgentCitizenshipRecord {
            schema: crate::AGENT_CITIZENSHIP_SCHEMA,
            agent_id: "hermes:main".to_string(),
            scope: AgentScope::Crate,
            budgets: AgentBudgets {
                actions,
                wall_seconds,
            },
            residual_risk: None,
            evidence: None,
        }
    }

    fn budgets(actions: u64, wall_seconds: u64) -> AgentBudgets {
        AgentBudgets {
            actions,
            wall_seconds,
        }
    }

    #[test]
    fn build_mutation_admits_when_role_matches() {
        let admission = admit_build_mutation(
            &record(50, 3600),
            Some(&AgentScope::Crate),
            &budgets(10, 600),
            &AgentScope::File,
            true,
            "build_runner",
            "build_runner",
        );
        assert!(admission.is_admitted());
    }

    #[test]
    fn build_mutation_refuses_when_role_mismatches() {
        let admission = admit_build_mutation(
            &record(50, 3600),
            Some(&AgentScope::Crate),
            &budgets(10, 600),
            &AgentScope::File,
            true,
            "build_runner",
            "kernel_primary",
        );
        assert!(!admission.is_admitted());
    }

    #[test]
    fn workload_mutation_admits_with_live_claim() {
        let admission = admit_workload_mutation(
            &record(50, 3600),
            Some(&AgentScope::Crate),
            &budgets(10, 600),
            &AgentScope::File,
            true,
            Some(999_999_999_999),
            100_000_000_000,
        );
        assert!(admission.is_admitted());
    }

    #[test]
    fn workload_mutation_refuses_expired_claim() {
        let admission = admit_workload_mutation(
            &record(50, 3600),
            Some(&AgentScope::Crate),
            &budgets(10, 600),
            &AgentScope::File,
            true,
            Some(100),
            999_999_999_999,
        );
        assert!(!admission.is_admitted());
    }

    #[test]
    fn workload_mutation_refuses_without_claim() {
        let admission = admit_workload_mutation(
            &record(50, 3600),
            Some(&AgentScope::Crate),
            &budgets(10, 600),
            &AgentScope::File,
            true,
            None,
            999_999_999_999,
        );
        assert!(!admission.is_admitted());
    }
}
