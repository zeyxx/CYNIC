use serde::{Deserialize, Serialize};

use cynic_protocol::evidence::{EvidenceRef, EvidenceRelation, valid_sha256_digest};

use super::{ConvergenceCell, ConvergenceStatus};

pub const CAUSAL_CONVERGENCE_EPOCH_SCHEMA: &str = "cynic.convergence_epoch.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustDimensionStatus {
    Satisfied,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustAssessment {
    pub authority: TrustDimensionStatus,
    pub authenticity: TrustDimensionStatus,
    pub integrity: TrustDimensionStatus,
    pub freshness: TrustDimensionStatus,
    pub scope: TrustDimensionStatus,
    pub replay_safety: TrustDimensionStatus,
    pub read_back: TrustDimensionStatus,
    pub independence: EvidenceRelation,
}

/// Per-vertical admission policy. Trust is a vector: a gate selects the
/// dimensions it needs instead of collapsing every claim into one global score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustRequirements {
    pub authority: bool,
    pub authenticity: bool,
    pub integrity: bool,
    pub freshness: bool,
    pub scope: bool,
    pub replay_safety: bool,
    pub read_back: bool,
    pub independence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalEvidenceRefs {
    pub desired_state: EvidenceRef,
    pub observations: Vec<EvidenceRef>,
    pub decision: EvidenceRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization: Option<EvidenceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_receipt: Option<EvidenceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_back: Option<EvidenceRef>,
}

/// One immutable point in the subject × vertical × epoch convergence space.
/// The causal phases live inside the cell so the third axis remains time rather
/// than becoming another mutable orchestration hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalConvergenceEpoch {
    pub schema: String,
    pub epoch_id: String,
    pub observed_at_unix_ms: u128,
    pub cell: ConvergenceCell,
    pub causal_evidence: CausalEvidenceRefs,
    pub trust_requirements: TrustRequirements,
    pub trust: TrustAssessment,
    pub residual_unknowns: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalConvergenceEpochInput {
    pub epoch_id: String,
    pub observed_at_unix_ms: u128,
    pub subject_id: String,
    pub vertical_id: String,
    pub requested_status: ConvergenceStatus,
    pub causal_evidence: CausalEvidenceRefs,
    pub trust_requirements: TrustRequirements,
    pub trust: TrustAssessment,
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
}

pub fn assess_causal_convergence_epoch(
    input: CausalConvergenceEpochInput,
) -> CausalConvergenceEpoch {
    let CausalConvergenceEpochInput {
        epoch_id,
        observed_at_unix_ms,
        subject_id,
        vertical_id,
        requested_status,
        causal_evidence,
        trust_requirements,
        trust,
        mut residual_unknowns,
        mut blockers,
    } = input;
    if epoch_id.trim().is_empty() {
        blockers.push("epoch_id_missing".to_string());
    }
    if observed_at_unix_ms == 0 {
        blockers.push("epoch_observed_at_missing".to_string());
    }
    if subject_id.trim().is_empty() {
        blockers.push("epoch_subject_missing".to_string());
    }
    if vertical_id.trim().is_empty() {
        blockers.push("epoch_vertical_missing".to_string());
    }
    if causal_evidence.observations.is_empty() {
        blockers.push("epoch_observation_missing".to_string());
    }

    let evidence = causal_evidence.all();
    if evidence
        .iter()
        .any(|evidence| evidence.subject != subject_id)
    {
        blockers.push("epoch_evidence_subject_mismatch".to_string());
    }
    if evidence
        .iter()
        .any(|evidence| evidence.kind.trim().is_empty())
    {
        blockers.push("epoch_evidence_kind_missing".to_string());
    }
    if evidence
        .iter()
        .any(|evidence| !valid_sha256_digest(&evidence.digest))
    {
        blockers.push("epoch_evidence_digest_invalid".to_string());
    }

    for (name, required, status) in trust.dimensions(&trust_requirements) {
        if !required {
            continue;
        }
        match status {
            TrustDimensionStatus::Blocked => blockers.push(format!("trust_{name}_blocked")),
            TrustDimensionStatus::Unknown => {
                residual_unknowns.push(format!("trust_{name}_unknown"))
            }
            TrustDimensionStatus::Satisfied => {}
        }
    }
    if trust_requirements.independence {
        match &trust.independence {
            EvidenceRelation::Independent => {}
            EvidenceRelation::Correlated { reasons } => blockers.push(format!(
                "trust_independence_correlated:{}",
                reasons.join(",")
            )),
            EvidenceRelation::Unknown { missing_evidence } => residual_unknowns.push(format!(
                "trust_independence_unknown:{}",
                missing_evidence.join(",")
            )),
        }
    }

    if requested_status == ConvergenceStatus::Complete && causal_evidence.read_back.is_none() {
        blockers.push("complete_epoch_read_back_missing".to_string());
    }

    blockers.sort();
    blockers.dedup();
    residual_unknowns.sort();
    residual_unknowns.dedup();
    let status = if !blockers.is_empty() {
        ConvergenceStatus::Blocked
    } else if !residual_unknowns.is_empty() {
        ConvergenceStatus::Unknown
    } else {
        requested_status
    };
    let cell = ConvergenceCell {
        subject_id,
        vertical_id,
        status,
        evidence_refs: causal_evidence.digests(),
        blockers: blockers.clone(),
    };

    CausalConvergenceEpoch {
        schema: CAUSAL_CONVERGENCE_EPOCH_SCHEMA.to_string(),
        epoch_id,
        observed_at_unix_ms,
        cell,
        causal_evidence,
        trust_requirements,
        trust,
        residual_unknowns,
        blockers,
    }
}

impl CausalEvidenceRefs {
    fn all(&self) -> Vec<&EvidenceRef> {
        let mut evidence = vec![&self.desired_state];
        evidence.extend(&self.observations);
        evidence.push(&self.decision);
        evidence.extend(self.authorization.as_ref());
        evidence.extend(self.action_receipt.as_ref());
        evidence.extend(self.read_back.as_ref());
        evidence
    }

    fn digests(&self) -> Vec<String> {
        let mut digests = self
            .all()
            .into_iter()
            .map(|evidence| evidence.digest.clone())
            .collect::<Vec<_>>();
        digests.sort();
        digests.dedup();
        digests
    }
}

impl TrustAssessment {
    fn dimensions(
        &self,
        requirements: &TrustRequirements,
    ) -> [(&'static str, bool, TrustDimensionStatus); 7] {
        [
            ("authority", requirements.authority, self.authority),
            ("authenticity", requirements.authenticity, self.authenticity),
            ("integrity", requirements.integrity, self.integrity),
            ("freshness", requirements.freshness, self.freshness),
            ("scope", requirements.scope, self.scope),
            (
                "replay_safety",
                requirements.replay_safety,
                self.replay_safety,
            ),
            ("read_back", requirements.read_back, self.read_back),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(marker: char) -> String {
        format!("sha256:{}", marker.to_string().repeat(64))
    }

    fn evidence(kind: &str, marker: char) -> EvidenceRef {
        EvidenceRef {
            kind: kind.to_string(),
            subject: "local-kernel/process".to_string(),
            digest: digest(marker),
            lineage: None,
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

    fn strict_trust_requirements() -> TrustRequirements {
        TrustRequirements {
            authority: true,
            authenticity: true,
            integrity: true,
            freshness: true,
            scope: true,
            replay_safety: true,
            read_back: true,
            independence: true,
        }
    }

    fn causal_evidence(read_back: bool) -> CausalEvidenceRefs {
        CausalEvidenceRefs {
            desired_state: evidence("desired_state", '1'),
            observations: vec![evidence("process_observation", '2')],
            decision: evidence("reconciliation_decision", '3'),
            authorization: None,
            action_receipt: None,
            read_back: read_back.then(|| evidence("process_read_back", '4')),
        }
    }

    #[test]
    fn complete_epoch_requires_post_decision_read_back() {
        let epoch = assess_causal_convergence_epoch(CausalConvergenceEpochInput {
            epoch_id: "epoch-1".to_string(),
            observed_at_unix_ms: 1,
            subject_id: "local-kernel/process".to_string(),
            vertical_id: "process".to_string(),
            requested_status: ConvergenceStatus::Complete,
            causal_evidence: causal_evidence(false),
            trust_requirements: strict_trust_requirements(),
            trust: satisfied_trust(),
            residual_unknowns: Vec::new(),
            blockers: Vec::new(),
        });

        assert_eq!(epoch.cell.status, ConvergenceStatus::Blocked);
        assert_eq!(
            epoch.blockers,
            vec!["complete_epoch_read_back_missing".to_string()]
        );
    }

    #[test]
    fn local_kernel_process_epoch_closes_with_independent_read_back() {
        let epoch = assess_causal_convergence_epoch(CausalConvergenceEpochInput {
            epoch_id: "epoch-1".to_string(),
            observed_at_unix_ms: 1,
            subject_id: "local-kernel/process".to_string(),
            vertical_id: "process".to_string(),
            requested_status: ConvergenceStatus::Complete,
            causal_evidence: causal_evidence(true),
            trust_requirements: strict_trust_requirements(),
            trust: satisfied_trust(),
            residual_unknowns: Vec::new(),
            blockers: Vec::new(),
        });

        assert_eq!(epoch.cell.status, ConvergenceStatus::Complete);
        assert_eq!(epoch.cell.evidence_refs.len(), 4);
        assert!(epoch.blockers.is_empty());
        assert!(epoch.residual_unknowns.is_empty());
    }

    #[test]
    fn unknown_trust_is_not_promoted_to_partial_success() {
        let mut trust = satisfied_trust();
        trust.authenticity = TrustDimensionStatus::Unknown;
        let epoch = assess_causal_convergence_epoch(CausalConvergenceEpochInput {
            epoch_id: "epoch-1".to_string(),
            observed_at_unix_ms: 1,
            subject_id: "local-kernel/process".to_string(),
            vertical_id: "process".to_string(),
            requested_status: ConvergenceStatus::Complete,
            causal_evidence: causal_evidence(true),
            trust_requirements: strict_trust_requirements(),
            trust,
            residual_unknowns: Vec::new(),
            blockers: Vec::new(),
        });

        assert_eq!(epoch.cell.status, ConvergenceStatus::Unknown);
        assert_eq!(
            epoch.residual_unknowns,
            vec!["trust_authenticity_unknown".to_string()]
        );
    }

    #[test]
    fn vertical_policy_does_not_require_irrelevant_trust_dimensions() {
        let mut trust = satisfied_trust();
        trust.authenticity = TrustDimensionStatus::Unknown;
        trust.independence = EvidenceRelation::Unknown {
            missing_evidence: vec!["second_producer".to_string()],
        };
        let mut requirements = strict_trust_requirements();
        requirements.authenticity = false;
        requirements.independence = false;

        let epoch = assess_causal_convergence_epoch(CausalConvergenceEpochInput {
            epoch_id: "epoch-1".to_string(),
            observed_at_unix_ms: 1,
            subject_id: "local-kernel/process".to_string(),
            vertical_id: "process".to_string(),
            requested_status: ConvergenceStatus::Complete,
            causal_evidence: causal_evidence(true),
            trust_requirements: requirements,
            trust,
            residual_unknowns: Vec::new(),
            blockers: Vec::new(),
        });

        assert_eq!(epoch.cell.status, ConvergenceStatus::Complete);
        assert!(epoch.residual_unknowns.is_empty());
    }
}
