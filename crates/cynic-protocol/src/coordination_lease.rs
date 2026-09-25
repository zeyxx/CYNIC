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
<<<<<<< HEAD

pub const HUMAN_SCOPED_FALLBACK_ADMISSION_SCHEMA: &str =
    "cynic.coordination.human_scoped_fallback_admission.v1";
pub const HUMAN_SCOPED_FALLBACK_PROPOSAL_SCHEMA: &str =
    "cynic.coordination.human_scoped_fallback_proposal.v1";
pub const FALLBACK_SLICE_RECEIPT_SCHEMA: &str = "cynic.coordination.fallback_slice_receipt.v1";

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

/// An inert proposal for a local source slice while the canonical coordination
/// runtime is demonstrably degraded. It is not a lease or execution authority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HumanScopedFallbackProposal {
    pub schema: String,
    pub runtime_availability: CoordinationRuntimeAvailability,
    pub source_target: String,
    pub allowed_paths: Vec<String>,
    pub baseline_ref: String,
    pub pre_worktree_digest: String,
    pub validation_target: String,
    pub isolation_profile_digest: String,
    pub claims_revision: u64,
    pub action_budget: u64,
    pub wall_seconds: u64,
    pub rollback_intent: String,
    pub evidence_refs: Vec<String>,
}

impl HumanScopedFallbackProposal {
    pub fn digest(&self) -> Result<String, serde_json::Error> {
        crate::digest_json(self)
    }

    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != HUMAN_SCOPED_FALLBACK_PROPOSAL_SCHEMA {
            blockers.push("human_scoped_fallback_proposal_schema_invalid".to_string());
        }
        if matches!(
            self.runtime_availability,
            CoordinationRuntimeAvailability::ActiveStable
                | CoordinationRuntimeAvailability::Unknown
        ) {
            blockers.push("human_scoped_fallback_proposal_runtime_not_degraded".to_string());
        }
        if !valid_relative_path(&self.source_target) {
            blockers.push("human_scoped_fallback_proposal_source_target_invalid".to_string());
        }
        if self.allowed_paths.is_empty()
            || self.allowed_paths.iter().any(|path| {
                !valid_relative_path(path) || !path_within_scope(path, &self.source_target)
            })
        {
            blockers.push("human_scoped_fallback_proposal_paths_invalid".to_string());
        }
        if self.baseline_ref.trim().is_empty()
            || !valid_sha256_digest(&self.pre_worktree_digest)
            || !valid_sha256_digest(&self.isolation_profile_digest)
            || self.validation_target.trim().is_empty()
            || self.rollback_intent.trim().is_empty()
            || self.evidence_refs.is_empty()
            || self
                .evidence_refs
                .iter()
                .any(|evidence_ref| evidence_ref.trim().is_empty())
        {
            blockers.push("human_scoped_fallback_proposal_evidence_incomplete".to_string());
        }
        if self.action_budget == 0 || self.wall_seconds == 0 {
            blockers.push("human_scoped_fallback_proposal_budget_required".to_string());
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

/// Read-back after a human-admitted source slice. A receipt records evidence;
/// it never promotes itself to a durable coordination lease.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FallbackSliceReceipt {
    pub schema: String,
    pub proposal_digest: String,
    pub modified_paths: Vec<String>,
    pub validation_target: String,
    pub validation_passed: bool,
    pub post_worktree_digest: String,
    pub consumed_actions: u64,
    pub elapsed_wall_seconds: u64,
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
}

pub fn validate_fallback_slice_receipt(
    proposal: &HumanScopedFallbackProposal,
    receipt: &FallbackSliceReceipt,
) -> Vec<String> {
    let mut blockers = proposal.validate();
    if receipt.schema != FALLBACK_SLICE_RECEIPT_SCHEMA {
        blockers.push("fallback_receipt_schema_invalid".to_string());
    }
    match proposal.digest() {
        Ok(digest) if digest == receipt.proposal_digest => {}
        _ => blockers.push("fallback_receipt_proposal_digest_mismatch".to_string()),
    }
    if receipt
        .modified_paths
        .iter()
        .any(|path| !proposal.allowed_paths.iter().any(|allowed| path == allowed))
    {
        blockers.push("fallback_receipt_modified_path_out_of_scope".to_string());
    }
    if receipt.validation_target != proposal.validation_target {
        blockers.push("fallback_receipt_validation_target_mismatch".to_string());
    }
    if !receipt.validation_passed {
        blockers.push("fallback_receipt_validation_not_passed".to_string());
    }
    if !valid_sha256_digest(&receipt.post_worktree_digest) {
        blockers.push("fallback_receipt_post_worktree_digest_invalid".to_string());
    }
    if receipt.consumed_actions > proposal.action_budget {
        blockers.push("fallback_receipt_budget_exceeded".to_string());
    }
    if receipt.elapsed_wall_seconds > proposal.wall_seconds {
        blockers.push("fallback_receipt_wall_time_exceeded".to_string());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn valid_relative_path(path: &str) -> bool {
    !path.trim().is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "..")
}

fn path_within_scope(path: &str, scope: &str) -> bool {
    path == scope
        || path
            .strip_prefix(scope)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn valid_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
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

    fn fallback_proposal() -> HumanScopedFallbackProposal {
        HumanScopedFallbackProposal {
            schema: HUMAN_SCOPED_FALLBACK_PROPOSAL_SCHEMA.to_string(),
            runtime_availability: CoordinationRuntimeAvailability::Incompatible,
            source_target: "crates/cynic-protocol/src/coordination_lease.rs".to_string(),
            allowed_paths: vec!["crates/cynic-protocol/src/coordination_lease.rs".to_string()],
            baseline_ref: "0123456789012345678901234567890123456789".to_string(),
            pre_worktree_digest:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
            validation_target: "cargo test -p cynic-protocol --lib coordination_lease".to_string(),
            isolation_profile_digest:
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_string(),
            claims_revision: 24,
            action_budget: 20,
            wall_seconds: 3600,
            rollback_intent: "discard the uncommitted local slice".to_string(),
            evidence_refs: vec!["runtime:mcp-observation".to_string()],
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

    #[test]
    fn fallback_receipt_rejects_out_of_scope_change_and_missing_validation() {
        let proposal = HumanScopedFallbackProposal {
            schema: HUMAN_SCOPED_FALLBACK_PROPOSAL_SCHEMA.to_string(),
            runtime_availability: CoordinationRuntimeAvailability::Incompatible,
            source_target: "crates/cynic-protocol/src/coordination_lease.rs".to_string(),
            allowed_paths: vec!["crates/cynic-protocol/src/coordination_lease.rs".to_string()],
            baseline_ref: "0123456789012345678901234567890123456789".to_string(),
            pre_worktree_digest:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
            validation_target: "cargo test -p cynic-protocol --lib coordination_lease".to_string(),
            isolation_profile_digest:
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_string(),
            claims_revision: 24,
            action_budget: 20,
            wall_seconds: 3600,
            rollback_intent: "discard the uncommitted local slice".to_string(),
            evidence_refs: vec!["runtime:mcp-observation".to_string()],
        };
        let receipt = FallbackSliceReceipt {
            schema: FALLBACK_SLICE_RECEIPT_SCHEMA.to_string(),
            proposal_digest: proposal.digest().expect("proposal is serializable"),
            modified_paths: vec!["crates/cynic-host/src/lib.rs".to_string()],
            validation_target: proposal.validation_target.clone(),
            validation_passed: false,
            post_worktree_digest:
                "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                    .to_string(),
            consumed_actions: 1,
            elapsed_wall_seconds: 1,
            residual_unknowns: Vec::new(),
        };

        let blockers = validate_fallback_slice_receipt(&proposal, &receipt);
        assert!(blockers.contains(&"fallback_receipt_modified_path_out_of_scope".to_string()));
        assert!(blockers.contains(&"fallback_receipt_validation_not_passed".to_string()));
    }

    #[test]
    fn valid_fallback_receipt_accepts_read_only_validation_and_rejects_unsafe_paths() {
        let mut proposal = fallback_proposal();
        let receipt = FallbackSliceReceipt {
            schema: FALLBACK_SLICE_RECEIPT_SCHEMA.to_string(),
            proposal_digest: proposal.digest().expect("proposal is serializable"),
            modified_paths: Vec::new(),
            validation_target: proposal.validation_target.clone(),
            validation_passed: true,
            post_worktree_digest:
                "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                    .to_string(),
            consumed_actions: 0,
            elapsed_wall_seconds: 1,
            residual_unknowns: Vec::new(),
        };
        assert!(validate_fallback_slice_receipt(&proposal, &receipt).is_empty());

        proposal.evidence_refs = vec![" ".to_string()];
        proposal.allowed_paths =
            vec!["crates\\cynic-protocol\\src\\coordination_lease.rs".to_string()];
        let blockers = proposal.validate();
        assert!(
            blockers.contains(&"human_scoped_fallback_proposal_evidence_incomplete".to_string())
        );
        assert!(blockers.contains(&"human_scoped_fallback_proposal_paths_invalid".to_string()));
    }
}
=======
>>>>>>> github/main
