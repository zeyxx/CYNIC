//! Pure harness progression decision — whether the system can advance
//! from one RuntimeHarnessKind to the next.
//!
//! From AGENTS.md Runtime Harness section:
//!   lab → conformance → staging → canary → production
//!
//! Each step requires explicit proof. No silent promotion.
use crate::runtime_harness::{HarnessAuthority, RuntimeHarnessKind};

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
    pub current_kind: RuntimeHarnessKind,
    pub target_kind: RuntimeHarnessKind,
    pub satisfied: Vec<ProofCategory>,
    pub pending: Vec<ProofCategory>,
    /// Residual unknowns — categories that could not be observed.
    pub unknown: Vec<ProofCategory>,
}

/// Pure decision: can the system progress from current to target harness kind?
///
/// Returns a structured verdict — never a silent yes.
pub fn decide_harness_progression(
    evidence: &HarnessProgressionEvidence,
) -> HarnessProgressionVerdict {
    // Step must be exactly +1 in the progression ladder.
    if !is_single_step(&evidence.current_kind, &evidence.target_kind) {
        return HarnessProgressionVerdict::Blocked {
            reason: "harness_progression_must_be_single_step".into(),
            detail: format!(
                "cannot jump from {:?} to {:?}",
                evidence.current_kind, evidence.target_kind
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
    let required = required_proof_categories(&evidence.target_kind);
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
        current_kind: evidence.current_kind.clone(),
        target_kind: evidence.target_kind.clone(),
        authority: authority_for_kind(&evidence.target_kind),
    }
}

/// Which proof categories are required to enter a given harness kind.
pub fn required_proof_categories(kind: &RuntimeHarnessKind) -> Vec<ProofCategory> {
    match kind {
        RuntimeHarnessKind::ClosedLocalLab => vec![],
        RuntimeHarnessKind::Conformance => {
            vec![ProofCategory::Bootstrap, ProofCategory::Hardware]
        }
        RuntimeHarnessKind::Staging => {
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
                ProofCategory::BuildReproduction,
                ProofCategory::Network,
            ]
        }
        RuntimeHarnessKind::Canary => {
            vec![
                ProofCategory::Bootstrap,
                ProofCategory::Hardware,
                ProofCategory::HostConformance,
                ProofCategory::BuildReproduction,
                ProofCategory::Network,
                ProofCategory::TargetAttestation,
            ]
        }
        RuntimeHarnessKind::Production => {
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

/// Authority granted at each harness level.
pub fn authority_for_kind(kind: &RuntimeHarnessKind) -> HarnessAuthority {
    match kind {
        RuntimeHarnessKind::ClosedLocalLab => HarnessAuthority::TestProjection,
        RuntimeHarnessKind::Conformance => HarnessAuthority::ConformanceGate,
        RuntimeHarnessKind::Staging => HarnessAuthority::ReleaseCandidate,
        RuntimeHarnessKind::Canary => HarnessAuthority::FleetCanary,
        RuntimeHarnessKind::Production => HarnessAuthority::RuntimeAuthority,
    }
}

/// Is the target exactly one step above the current kind?
fn is_single_step(current: &RuntimeHarnessKind, target: &RuntimeHarnessKind) -> bool {
    use RuntimeHarnessKind::*;
    matches!(
        (current, target),
        (ClosedLocalLab, Conformance)
            | (Conformance, Staging)
            | (Staging, Canary)
            | (Canary, Production)
    )
}

/// Progression verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessProgressionVerdict {
    /// Progression is allowed — target authority granted.
    Allowed {
        current_kind: RuntimeHarnessKind,
        target_kind: RuntimeHarnessKind,
        authority: HarnessAuthority,
    },
    /// Progression is blocked — missing proof or invalid step.
    Blocked { reason: String, detail: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use RuntimeHarnessKind::*;

    fn evidence(
        current: RuntimeHarnessKind,
        target: RuntimeHarnessKind,
        satisfied: Vec<ProofCategory>,
        unknown: Vec<ProofCategory>,
    ) -> HarnessProgressionEvidence {
        HarnessProgressionEvidence {
            current_kind: current,
            target_kind: target,
            satisfied,
            pending: vec![],
            unknown,
        }
    }

    #[test]
    fn lab_to_conformance_allowed_with_bootstrap_and_hardware() {
        let ev = evidence(
            ClosedLocalLab,
            Conformance,
            vec![ProofCategory::Bootstrap, ProofCategory::Hardware],
            vec![],
        );
        let verdict = decide_harness_progression(&ev);
        assert!(
            matches!(&verdict, HarnessProgressionVerdict::Allowed { target_kind, .. } if *target_kind == Conformance),
            "expected Allowed, got {:?}",
            verdict
        );
    }

    #[test]
    fn lab_to_conformance_blocked_without_hardware() {
        let ev = evidence(
            ClosedLocalLab,
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
        assert!(matches!(&verdict, HarnessProgressionVerdict::Allowed { .. }));
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
            matches!(&verdict, HarnessProgressionVerdict::Allowed { authority, .. } if *authority == HarnessAuthority::RuntimeAuthority)
        );
    }

    #[test]
    fn multi_step_jump_blocked() {
        let ev = evidence(
            ClosedLocalLab,
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
            ClosedLocalLab,
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
        assert!(matches!(&verdict, HarnessProgressionVerdict::Blocked { .. }));
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
