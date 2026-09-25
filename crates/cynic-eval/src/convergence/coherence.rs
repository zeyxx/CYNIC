//! Shared Compass/Vector coherence primitive.
//!
//! The CYNIC coherence pattern (AGENTS.md) splits every reconciliation into:
//! - COMPASS = DIRECTION — the desired state, comparable without mutation.
//! - VECTOR  = FORCE     — the pure decision plus the bounded action and its
//!   independent read-back proof (the magnitude applied to the substrate).
//!
//! `cynic-host::process_convergence` already drives `CausalConvergenceEpoch`
//! directly. This module lifts that shape into a reusable primitive so every
//! `reconcile_*` in cynic-protocol can emit the same proof instead of
//! re-inventing {validate, compare, decide, plan, blockers} per vertical
//! (structural-invariants rule #6: one canonical decision source per concern).

use serde::{Deserialize, Serialize};

use cynic_protocol::evidence::EvidenceRef;
use cynic_protocol::integrity::digest_json;

use super::{
    CausalConvergenceEpoch, CausalConvergenceEpochInput, CausalEvidenceRefs, ConvergenceStatus,
    TrustAssessment, TrustRequirements, assess_causal_convergence_epoch,
};

pub const COHERENCE_VECTOR_SCHEMA: &str = "cynic.coherence_vector.v1";

/// COMPASS: the direction. The signed, comparable desired state — never
/// mutated by the reconcile step itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compass {
    pub schema: String,
    pub desired_state_digest: String,
    pub subject_id: String,
    pub vertical_id: String,
}

/// VECTOR: the force. The pure decision plus the bounded action and its
/// independent read-back proof (the magnitude applied to the substrate).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vector {
    pub decision: String,
    pub action: Option<String>,
    pub read_back_digest: Option<String>,
}

/// One compass + vector pair: the full coherence record for a single
/// desired/observed comparison. This is the canonical output every
/// `reconcile_*` should emit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoherenceVector {
    pub schema: String,
    pub compass: Compass,
    pub vector: Vector,
    pub status: ConvergenceStatus,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

/// Shared, canonical decision vocabulary. Vertical-specific enums
/// (`NodeControlDecision`, `AgentSchedulerStepDecision`, ...) should project
/// from this instead of re-declaring the same states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum CoherenceDecision {
    Converged,
    Planned { action: String },
    Paused,
    Unknown { missing_evidence: Vec<String> },
    Blocked { violations: Vec<String> },
}

impl CoherenceDecision {
    pub fn label(&self) -> String {
        match self {
            CoherenceDecision::Converged => "converged",
            CoherenceDecision::Planned { .. } => "planned",
            CoherenceDecision::Paused => "paused",
            CoherenceDecision::Unknown { .. } => "unknown",
            CoherenceDecision::Blocked { .. } => "blocked",
        }
        .to_string()
    }
}

fn evidence_ref(kind: &str, subject: &str, digest: &str) -> EvidenceRef {
    EvidenceRef {
        kind: kind.to_string(),
        subject: subject.to_string(),
        digest: digest.to_string(),
        lineage: None,
    }
}

/// Build a `CausalConvergenceEpoch` from the compass/vector shape.
///
/// `requested_status` is the optimistic target derived from the pure decision.
/// `observation_digest` is the independently observed state (the substrate read
/// back before/after action); if `None`, the epoch honestly reports a missing
/// observation blocker. If `vector.read_back_digest` is `None` but the decision
/// would be `Complete`, the epoch is honestly downgraded to `Unknown`: a
/// complete claim without independent read-back is not convergence — it is a
/// missing proof, not a success. This enforces the structural-invariant rule
/// that unknown is data.
#[allow(clippy::too_many_arguments)]
pub fn assess_coherence_vector(
    epoch_id: &str,
    observed_at_unix_ms: u128,
    compass: &Compass,
    vector: &Vector,
    observation_digest: Option<String>,
    requested_status: ConvergenceStatus,
    trust_requirements: TrustRequirements,
    trust: TrustAssessment,
    mut residual_unknowns: Vec<String>,
    blockers: Vec<String>,
) -> CausalConvergenceEpoch {
    let read_back = vector
        .read_back_digest
        .as_ref()
        .map(|digest| evidence_ref("read_back", &compass.subject_id, digest));
    let observed_refs: Vec<EvidenceRef> = observation_digest
        .map(|digest| vec![evidence_ref("observation", &compass.subject_id, &digest)])
        .unwrap_or_default();
    let decision_digest =
        digest_json(&vector.decision).unwrap_or_else(|_| format!("sha256:{}", "0".repeat(64)));
    let decision_ref = evidence_ref(
        "reconciliation_decision",
        &compass.subject_id,
        &decision_digest,
    );

    // Honest downgrade: completeness without independent read-back is unknown.
    let requested = if requested_status == ConvergenceStatus::Complete && read_back.is_none() {
        residual_unknowns.push("coherence_independent_read_back_missing".to_string());
        ConvergenceStatus::Unknown
    } else {
        requested_status
    };

    assess_causal_convergence_epoch(CausalConvergenceEpochInput {
        epoch_id: epoch_id.to_string(),
        observed_at_unix_ms,
        subject_id: compass.subject_id.clone(),
        vertical_id: compass.vertical_id.clone(),
        requested_status: requested,
        causal_evidence: CausalEvidenceRefs {
            desired_state: evidence_ref(
                "desired_state",
                &compass.subject_id,
                &compass.desired_state_digest,
            ),
            observations: observed_refs,
            decision: decision_ref,
            authorization: None,
            action_receipt: vector
                .action
                .as_ref()
                .map(|a| evidence_ref("action_receipt", &compass.subject_id, a)),
            read_back,
        },
        trust_requirements,
        trust,
        residual_unknowns,
        blockers,
    })
}

// ── Shared predicates (dedupe targets for Phase 2) ─────────────────────

pub fn check(condition: bool, violation: &str, violations: &mut Vec<String>) {
    if !condition {
        violations.push(violation.to_string());
    }
}

pub fn missing(desired: &[String], observed: &[String]) -> Vec<String> {
    let mut values = desired
        .iter()
        .filter(|value| !observed.contains(value))
        .cloned()
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

pub fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convergence::TrustDimensionStatus;
    use cynic_protocol::evidence::EvidenceRelation;

    fn compass() -> Compass {
        Compass {
            schema: COHERENCE_VECTOR_SCHEMA.to_string(),
            desired_state_digest: format!("sha256:{}", "a".repeat(64)),
            subject_id: "node-a".to_string(),
            vertical_id: "node_control".to_string(),
        }
    }

    fn satisfied_trust() -> TrustAssessment {
        TrustAssessment {
            authority: TrustDimensionStatus::Satisfied,
            authenticity: TrustDimensionStatus::Satisfied,
            integrity: TrustDimensionStatus::Satisfied,
            freshness: TrustDimensionStatus::Satisfied,
            scope: TrustDimensionStatus::Satisfied,
            replay_safety: TrustDimensionStatus::Satisfied,
            read_back: TrustDimensionStatus::Satisfied,
            independence: EvidenceRelation::Independent,
        }
    }

    fn trust_requirements(read_back: bool) -> TrustRequirements {
        TrustRequirements {
            authority: true,
            authenticity: false,
            integrity: true,
            freshness: true,
            scope: true,
            replay_safety: false,
            read_back,
            independence: false,
        }
    }

    #[test]
    fn converged_without_read_back_is_honestly_unknown() {
        let vector = Vector {
            decision: "converged".to_string(),
            action: None,
            read_back_digest: None,
        };
        let epoch = assess_coherence_vector(
            "epoch-1",
            1,
            &compass(),
            &vector,
            Some(format!("sha256:{}", "b".repeat(64))),
            ConvergenceStatus::Complete,
            trust_requirements(false),
            satisfied_trust(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(epoch.cell.status, ConvergenceStatus::Unknown);
        assert!(
            epoch
                .residual_unknowns
                .contains(&"coherence_independent_read_back_missing".to_string())
        );
    }

    #[test]
    fn converged_with_read_back_closes_epoch() {
        let vector = Vector {
            decision: "converged".to_string(),
            action: None,
            read_back_digest: Some(format!("sha256:{}", "b".repeat(64))),
        };
        let epoch = assess_coherence_vector(
            "epoch-1",
            1,
            &compass(),
            &vector,
            Some(format!("sha256:{}", "c".repeat(64))),
            ConvergenceStatus::Complete,
            trust_requirements(true),
            satisfied_trust(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(epoch.cell.status, ConvergenceStatus::Complete);
        assert!(epoch.blockers.is_empty());
    }

    #[test]
    fn blocked_decision_produces_blocked_epoch() {
        let vector = Vector {
            decision: "blocked".to_string(),
            action: None,
            read_back_digest: None,
        };
        let epoch = assess_coherence_vector(
            "epoch-1",
            1,
            &compass(),
            &vector,
            Some(format!("sha256:{}", "c".repeat(64))),
            ConvergenceStatus::Blocked,
            trust_requirements(false),
            satisfied_trust(),
            Vec::new(),
            vec!["node_capability_not_granted:build.execute".to_string()],
        );
        assert_eq!(epoch.cell.status, ConvergenceStatus::Blocked);
        assert!(
            epoch
                .blockers
                .contains(&"node_capability_not_granted:build.execute".to_string())
        );
    }

    #[test]
    fn missing_observation_is_blocked_not_unknown() {
        let vector = Vector {
            decision: "converged".to_string(),
            action: None,
            read_back_digest: None,
        };
        let epoch = assess_coherence_vector(
            "epoch-1",
            1,
            &compass(),
            &vector,
            None,
            ConvergenceStatus::Complete,
            trust_requirements(false),
            satisfied_trust(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(epoch.cell.status, ConvergenceStatus::Blocked);
        assert!(
            epoch
                .blockers
                .contains(&"epoch_observation_missing".to_string())
        );
    }
}
