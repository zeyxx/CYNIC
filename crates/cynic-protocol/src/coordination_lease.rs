use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeaseGrant {
    pub lease_id: String,
    pub fencing_token: u64,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeaseGuard {
    pub agent_id: String,
    pub target: String,
    pub lease_id: String,
    pub fencing_token: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LeaseAction {
    Renew,
    Release,
    Finalize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LeaseRejectionCode {
    GuardRejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LeaseMutationOutcome {
    Applied {
        action: LeaseAction,
        target: String,
        fencing_token: u64,
        expires_at: Option<String>,
        proof_ref: Option<String>,
    },
    Rejected {
        action: LeaseAction,
        target: String,
        fencing_token: u64,
        code: LeaseRejectionCode,
    },
}

impl LeaseMutationOutcome {
    pub fn applied(
        action: LeaseAction,
        target: impl Into<String>,
        fencing_token: u64,
        expires_at: Option<String>,
        proof_ref: Option<String>,
    ) -> Self {
        Self::Applied {
            action,
            target: target.into(),
            fencing_token,
            expires_at,
            proof_ref,
        }
    }

    pub fn rejected(action: LeaseAction, target: impl Into<String>, fencing_token: u64) -> Self {
        Self::Rejected {
            action,
            target: target.into(),
            fencing_token,
            code: LeaseRejectionCode::GuardRejected,
        }
    }

    pub fn is_applied(&self) -> bool {
        matches!(self, Self::Applied { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeaseConflict {
    pub agent_id: String,
    pub claimed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum TargetClaimOutcome {
    Claimed {
        target: String,
        grant: LeaseGrant,
    },
    Conflict {
        target: String,
        conflicts: Vec<LeaseConflict>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchClaimResult {
    pub outcomes: Vec<TargetClaimOutcome>,
}

impl BatchClaimResult {
    pub fn claimed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| matches!(outcome, TargetClaimOutcome::Claimed { .. }))
            .count()
    }

    pub fn conflict_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| matches!(outcome, TargetClaimOutcome::Conflict { .. }))
            .count()
    }
}

pub const HUMAN_SCOPED_FALLBACK_ADMISSION_SCHEMA: &str =
    "cynic.coordination.human_scoped_fallback_admission.v1";

/// Read-back classification of the deployed coordination runtime. Only a
/// demonstrably unavailable, incompatible, or unstable runtime can open the
/// human-scoped source fallback; an active stable runtime must remain fenced.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationRuntimeAvailability {
    ActiveStable,
    Unavailable,
    Incompatible,
    Unstable,
    Unknown,
}

/// The fallback deliberately excludes host, network, fleet, release, and
/// credential mutation. It is a source-repair escape hatch, not a second
/// runtime execution authority.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HumanScopedFallbackAction {
    LocalSourceMutation,
    LocalSourceValidation,
    RuntimeMutation,
    FleetMutation,
    ReleaseMutation,
    CredentialMutation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationMode {
    RuntimeFenced,
    HumanScopedFallback,
}

/// Explicit, bounded human authorization used only while coordination binaries
/// cannot provide a stable, observable lease. `claim_conflict_observed` is a
/// read-back fact, not a lease substitute.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HumanScopedFallbackAdmission {
    pub schema: String,
    pub runtime_availability: CoordinationRuntimeAvailability,
    pub human_approved: bool,
    pub action: HumanScopedFallbackAction,
    pub source_target: String,
    pub action_budget: u64,
    pub wall_seconds: u64,
    pub claim_conflict_observed: bool,
}

pub fn decide_coordination_mode(
    admission: &HumanScopedFallbackAdmission,
) -> Result<CoordinationMode, Vec<String>> {
    let mut blockers = Vec::new();
    if admission.schema != HUMAN_SCOPED_FALLBACK_ADMISSION_SCHEMA {
        blockers.push("human_scoped_fallback_schema_invalid".to_string());
    }
    if !admission.human_approved {
        blockers.push("human_scoped_fallback_human_approval_required".to_string());
    }
    if admission.source_target.trim().is_empty()
        || admission.source_target.starts_with('/')
        || admission
            .source_target
            .split('/')
            .any(|segment| segment == "..")
    {
        blockers.push("human_scoped_fallback_source_target_invalid".to_string());
    }
    if admission.action_budget == 0 || admission.wall_seconds == 0 {
        blockers.push("human_scoped_fallback_budget_required".to_string());
    }
    if admission.claim_conflict_observed {
        blockers.push("human_scoped_fallback_claim_conflict_observed".to_string());
    }
    if matches!(
        admission.action,
        HumanScopedFallbackAction::RuntimeMutation
            | HumanScopedFallbackAction::FleetMutation
            | HumanScopedFallbackAction::ReleaseMutation
            | HumanScopedFallbackAction::CredentialMutation
    ) {
        blockers.push("human_scoped_fallback_action_forbidden".to_string());
    }
    match admission.runtime_availability {
        CoordinationRuntimeAvailability::ActiveStable => {
            return Ok(CoordinationMode::RuntimeFenced);
        }
        CoordinationRuntimeAvailability::Unknown => {
            blockers.push("human_scoped_fallback_runtime_availability_unproven".to_string())
        }
        CoordinationRuntimeAvailability::Unavailable
        | CoordinationRuntimeAvailability::Incompatible
        | CoordinationRuntimeAvailability::Unstable => {}
    }
    if blockers.is_empty() {
        Ok(CoordinationMode::HumanScopedFallback)
    } else {
        blockers.sort();
        blockers.dedup();
        Err(blockers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admission() -> HumanScopedFallbackAdmission {
        HumanScopedFallbackAdmission {
            schema: HUMAN_SCOPED_FALLBACK_ADMISSION_SCHEMA.to_string(),
            runtime_availability: CoordinationRuntimeAvailability::Unavailable,
            human_approved: true,
            action: HumanScopedFallbackAction::LocalSourceMutation,
            source_target: "crates/cynic-host/src/ssh_trust".to_string(),
            action_budget: 20,
            wall_seconds: 1_800,
            claim_conflict_observed: false,
        }
    }

    #[test]
    fn unavailable_runtime_with_bounded_human_source_scope_is_admitted() {
        assert_eq!(
            decide_coordination_mode(&admission()),
            Ok(CoordinationMode::HumanScopedFallback)
        );
    }

    #[test]
    fn stable_runtime_requires_the_fenced_path() {
        let mut request = admission();
        request.runtime_availability = CoordinationRuntimeAvailability::ActiveStable;
        assert_eq!(
            decide_coordination_mode(&request),
            Ok(CoordinationMode::RuntimeFenced)
        );
    }

    #[test]
    fn fallback_rejects_missing_human_approval_conflicts_and_runtime_mutation() {
        let mut request = admission();
        request.human_approved = false;
        request.claim_conflict_observed = true;
        request.action = HumanScopedFallbackAction::RuntimeMutation;
        let blockers = decide_coordination_mode(&request).expect_err("must reject");
        assert!(blockers.contains(&"human_scoped_fallback_human_approval_required".to_string()));
        assert!(blockers.contains(&"human_scoped_fallback_claim_conflict_observed".to_string()));
        assert!(blockers.contains(&"human_scoped_fallback_action_forbidden".to_string()));
    }

    #[test]
    fn fallback_rejects_an_unbounded_source_target_or_unproven_runtime_state() {
        let mut request = admission();
        request.source_target = "../outside-worktree".to_string();
        request.runtime_availability = CoordinationRuntimeAvailability::Unknown;
        let blockers = decide_coordination_mode(&request).expect_err("must reject");
        assert!(blockers.contains(&"human_scoped_fallback_source_target_invalid".to_string()));
        assert!(
            blockers.contains(&"human_scoped_fallback_runtime_availability_unproven".to_string())
        );
    }
}
