//! Pure harness progression decision — whether the system can advance
//! from one DeploymentTier to the next.
//!
//! From AGENTS.md Runtime Harness section:
//!   lab → conformance → staging → canary → production
//!
//! Each step requires explicit proof. No silent promotion.
use cynic_protocol::build::Authority;
use serde::Serialize;

pub use cynic_protocol::deployment_tier::{
    DeploymentTier, ProofCategory as TierProofCategory,
    TierProgressionEvidence, TierProgressionVerdict,
};

/// Required proof categories for harness progression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofCategory {
    /// Bootstrap contracts satisfied (security, execution, observation).
    Bootstrap,
    /// Hardware baseline matches declared profile.
    Hardware,
    /// Host conformance report is green.
    HostConformance,
    /// Build evidence is reproducible.
    BuildReproduction,
    /// Network preflight verified.
    Network,
    /// Target attestation verified for the next boundary.
    TargetAttestation,
    /// Rollback intent declared.
    RollbackIntent,
}

/// Evidence snapshot for progression evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessProgressionEvidence {
    pub current_tier: DeploymentTier,
    pub target_tier: DeploymentTier,
    pub satisfied: Vec<ProofCategory>,
    pub pending: Vec<ProofCategory>,
    /// Residual unknowns — categories that could not be observed.
    pub unknown: Vec<ProofCategory>,
}

/// Pure decision: can the system progress from current to target tier?
///
/// Returns a structured verdict — never a silent yes.
pub fn decide_harness_progression(
    evidence: &HarnessProgressionEvidence,
) -> HarnessProgressionVerdict {
    // Step must be exactly +1 in the progression ladder.
    if !is_single_step(&evidence.current_tier, &evidence.target_tier) {
        return HarnessProgressionVerdict::Blocked {
            reason: "harness_progression_must_be_single_step".into(),
            detail: format!(
                "cannot jump from {:?} to {:?}",
                evidence.current_tier, evidence.target_tier
            ),
        };
    }

    // Unknown categories block progression (unknown ≠ satisfied).
    if !evidence.unknown.is_empty() {
        return HarnessProgressionVerdict::Blocked {
            reason: "harness_progression_unknown_proof".into(),
            detail: format!(
                "proof categories could not be observed: {:?}",
                evidence.unknown
            ),
        };
    }

    // Missing required categories block progression.
    let required = required_proof_categories(&evidence.target_tier);
    let missing: Vec<_> = required
        .iter()
        .filter(|cat| !evidence.satisfied.contains(cat))
        .cloned()
        .collect();

    if !missing.is_empty() {
        return HarnessProgressionVerdict::Blocked {
            reason: "harness_progression_missing_proof".into(),
            detail: format!("missing proof categories: {:?}", missing),
        };
    }

    HarnessProgressionVerdict::Allowed {
        current_tier: evidence.current_tier.clone(),
        target_tier: evidence.target_tier.clone(),
        authority: authority_for_tier(&evidence.target_tier),
    }
}

/// Which proof categories are required to enter a given tier.
pub fn required_proof_categories(tier: &DeploymentTier) -> Vec<ProofCategory> {
    match tier {
        DeploymentTier::LocalLab => vec![],
        DeploymentTier::Conformance => {
            vec![ProofCategory::Bootstrap, ProofCategory::Hardware]
        }
        DeploymentTier::Staging => {
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
                ProofCategory::BuildReproduction,
                ProofCategory::Network,
            ]
        }
        DeploymentTier::Canary => {
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
                ProofCategory::BuildReproduction,
                ProofCategory::Network,
                ProofCategory::TargetAttestation,
            ]
        }
        DeploymentTier::Production => {
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
                ProofCategory::BuildReproduction,
                ProofCategory::Network,
                ProofCategory::TargetAttestation,
                ProofCategory::RollbackIntent,
            ]
        }
    }
}

/// Authority granted at each tier level.
pub fn authority_for_tier(tier: &DeploymentTier) -> Authority {
    match tier {
        DeploymentTier::LocalLab => Authority::TestProjection,
        DeploymentTier::Conformance => Authority::ConformanceGate,
        DeploymentTier::Staging => Authority::ReleaseCandidate,
        DeploymentTier::Canary => Authority::FleetCanary,
        DeploymentTier::Production => Authority::RuntimeAuthority,
    }
}

/// Is the target exactly one step above the current tier?
fn is_single_step(current: &DeploymentTier, target: &DeploymentTier) -> bool {
    use DeploymentTier::*;
    matches!(
        (current, target),
        (LocalLab, Conformance)
            | (Conformance, Staging)
            | (Staging, Canary)
            | (Canary, Production)
    )
}

/// Progression verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum HarnessProgressionVerdict {
    /// Progression is allowed — target authority granted.
    Allowed {
        current_tier: DeploymentTier,
        target_tier: DeploymentTier,
        authority: Authority,
    },
    /// Progression is blocked — missing proof or invalid step.
    Blocked { reason: String, detail: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use DeploymentTier::*;

    fn evidence(
        current: DeploymentTier,
        target: DeploymentTier,
        satisfied: Vec<ProofCategory>,
        unknown: Vec<ProofCategory>,
    ) -> HarnessProgressionEvidence {
        HarnessProgressionEvidence {
            current_tier: current,
            target_tier: target,
            satisfied,
            pending: vec![],
            unknown,
        }
    }

    #[test]
    fn lab_to_conformance_allowed_with_bootstrap_and_hardware() {
        let ev = evidence(
            LocalLab,
            Conformance,
            vec![ProofCategory::Bootstrap, ProofCategory::Hardware],
            vec![],
        );
        let verdict = decide_harness_progression(&ev);
        assert!(
            matches!(&verdict, HarnessProgressionVerdict::Allowed { target_tier, .. } if *target_tier == Conformance),
            "expected Allowed, got {:?}",
            verdict
        );
    }

    #[test]
    fn lab_to_conformance_blocked_without_hardware() {
        let ev = evidence(
            LocalLab,
            Conformance,
            vec![ProofCategory::Bootstrap],
            vec![],
        );
        let verdict = decide_harness_progression(&ev);
        assert!(
            matches!(&verdict, HarnessProgressionVerdict::Blocked { reason, .. } if reason == "harness_progression_missing_proof"),
            "expected Blocked(missing_proof), got {:?}",
            verdict
        );
    }

    #[test]
    fn conformance_to_staging_requires_five_categories() {
        let ev = evidence(
            Conformance,
            Staging,
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
                ProofCategory::BuildReproduction,
                ProofCategory::Network,
            ],
            vec![],
        );
        let verdict = decide_harness_progression(&ev);
        assert!(matches!(
            &verdict,
            HarnessProgressionVerdict::Allowed { .. }
        ));
    }

    #[test]
    fn production_requires_rollback_intent() {
        let ev = evidence(
            Canary,
            Production,
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
                ProofCategory::BuildReproduction,
                ProofCategory::Network,
                ProofCategory::TargetAttestation,
                ProofCategory::RollbackIntent,
            ],
            vec![],
        );
        let verdict = decide_harness_progression(&ev);
        assert!(
            matches!(&verdict, HarnessProgressionVerdict::Allowed { authority, .. } if *authority == Authority::RuntimeAuthority)
        );
    }

    #[test]
    fn multi_step_jump_blocked() {
        let ev = evidence(
            LocalLab,
            Production,
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
                ProofCategory::BuildReproduction,
                ProofCategory::Network,
                ProofCategory::TargetAttestation,
                ProofCategory::RollbackIntent,
            ],
            vec![],
        );
        let verdict = decide_harness_progression(&ev);
        assert!(
            matches!(&verdict, HarnessProgressionVerdict::Blocked { reason, .. } if reason == "harness_progression_must_be_single_step"),
            "expected Blocked(single_step), got {:?}",
            verdict
        );
    }

    #[test]
    fn unknown_categories_block_progression() {
        let ev = evidence(
            LocalLab,
            Conformance,
            vec![ProofCategory::Bootstrap],
            vec![ProofCategory::Hardware],
        );
        let verdict = decide_harness_progression(&ev);
        assert!(
            matches!(&verdict, HarnessProgressionVerdict::Blocked { reason, .. } if reason == "harness_progression_unknown_proof"),
            "expected Blocked(unknown_proof), got {:?}",
            verdict
        );
    }

    #[test]
    fn same_kind_blocked_not_single_step() {
        let ev = evidence(
            Conformance,
            Conformance,
            vec![ProofCategory::Bootstrap, ProofCategory::Hardware],
            vec![],
        );
        let verdict = decide_harness_progression(&ev);
        assert!(matches!(
            &verdict,
            HarnessProgressionVerdict::Blocked { .. }
        ));
    }

    #[test]
    fn backward_step_blocked() {
        let ev = evidence(
            Staging,
            Conformance,
            vec![ProofCategory::Bootstrap, ProofCategory::Hardware],
            vec![],
        );
        let verdict = decide_harness_progression(&ev);
        assert!(
            matches!(&verdict, HarnessProgressionVerdict::Blocked { reason, .. } if reason == "harness_progression_must_be_single_step"),
            "expected Blocked(single_step), got {:?}",
            verdict
        );
    }
}
