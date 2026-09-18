//! Pure business logic evaluating release candidate compliance and relay eligibility.

use std::time::SystemTime;

use crate::contracts::{RelayPolicyContract, ReleaseGatesContract};
use crate::types::{
    GateEvaluationReport, GateProofRecord, RelayActionKind, RelayDecisionProof, ReleaseCandidate,
    ReleaseEligibilityVerdict, ReleaseVersion,
};

/// Evaluate whether a release candidate satisfies mandatory gates and version ordering.
pub fn evaluate_release_candidate(
    candidate: &ReleaseCandidate,
    gates_contract: &ReleaseGatesContract,
    proofs: &[GateProofRecord],
    prior_version: Option<&ReleaseVersion>,
) -> ReleaseEligibilityVerdict {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut blockers = Vec::new();

    // 1. Monotonicity check
    if let Some(prior) = prior_version {
        if !candidate
            .proposed_version
            .is_strictly_monotonic_after(prior)
        {
            blockers.push(format!(
                "Non-monotonic version bump: proposed {} is not strictly greater than prior {}",
                candidate.proposed_version.to_tag(),
                prior.to_tag()
            ));
        }
    }

    // 2. Mandatory gate proofs check
    for mandatory in &gates_contract.gates.mandatory {
        let matching_proof = proofs.iter().find(|p| p.name == mandatory.name);
        match matching_proof {
            Some(proof) => {
                if !proof.passed {
                    blockers.push(format!(
                        "Mandatory gate '{}' failed: {}",
                        mandatory.name, proof.detail
                    ));
                }
            }
            None => {
                blockers.push(format!(
                    "Missing required proof for mandatory gate '{}'",
                    mandatory.name
                ));
            }
        }
    }

    if blockers.is_empty() {
        ReleaseEligibilityVerdict::Eligible {
            schema: "cynic.release_verdict.v1",
            version: candidate.proposed_version.clone(),
            commit_sha: candidate.commit_sha.clone(),
            tag_name: candidate.proposed_version.to_tag(),
            decided_at_unix_s: now,
        }
    } else {
        ReleaseEligibilityVerdict::Blocked {
            schema: "cynic.release_verdict.v1",
            reasons: blockers,
            decided_at_unix_s: now,
        }
    }
}

/// Evaluate raw gate proof records against mandatory gate outcomes.
pub fn evaluate_gate_records(records: Vec<GateProofRecord>) -> GateEvaluationReport {
    let mut blockers = Vec::new();
    let mut all_passed = true;
    for r in &records {
        if !r.passed {
            all_passed = false;
            blockers.push(format!("Gate '{}' failed: {}", r.name, r.detail));
        }
    }
    GateEvaluationReport {
        schema: "cynic.release.gate_evaluation_report.v1",
        all_passed: all_passed && !records.is_empty(),
        records,
        blockers,
    }
}

/// Evaluate whether an eligible release is authorized for relay projection to GitHub.
pub fn evaluate_relay_eligibility(
    verdict: &ReleaseEligibilityVerdict,
    _relay_contract: &RelayPolicyContract,
    target_remote: &str,
    target_ref: &str,
    is_divergent: bool,
) -> RelayDecisionProof {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    match verdict {
        ReleaseEligibilityVerdict::Eligible {
            commit_sha,
            tag_name,
            ..
        } => {
            let action_kind = if is_divergent {
                RelayActionKind::MainForcedPromotion
            } else if target_ref.starts_with("refs/tags/") {
                RelayActionKind::TagPush
            } else {
                RelayActionKind::MainFastForward
            };

            RelayDecisionProof::Authorized {
                schema: "cynic.relay_decision_proof.v1",
                sovereign_commit: commit_sha.clone(),
                target_remote: target_remote.to_string(),
                target_ref: if target_ref.starts_with("refs/tags/") {
                    format!("refs/tags/{}", tag_name)
                } else {
                    target_ref.to_string()
                },
                action_kind,
                decided_at_unix_s: now,
            }
        }
        ReleaseEligibilityVerdict::Blocked { reasons, .. } => RelayDecisionProof::Rejected {
            schema: "cynic.relay_decision_proof.v1",
            blockers: reasons.clone(),
            decided_at_unix_s: now,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{GateDefinition, GatesCatalog, RelayActionSpec, RelayTopologySpec};

    fn dummy_contract() -> ReleaseGatesContract {
        ReleaseGatesContract {
            schema: "test".to_string(),
            status: "normative".to_string(),
            gates: GatesCatalog {
                mandatory: vec![
                    GateDefinition {
                        name: "cargo_check".to_string(),
                        description: "Compiles".to_string(),
                        command: "cargo check".to_string(),
                        pass_condition: "0".to_string(),
                    },
                    GateDefinition {
                        name: "secret_lint".to_string(),
                        description: "Clean".to_string(),
                        command: "cynic secret lint".to_string(),
                        pass_condition: "0".to_string(),
                    },
                ],
                optional: vec![],
            },
        }
    }

    fn dummy_relay_contract() -> RelayPolicyContract {
        RelayPolicyContract {
            schema: "cynic.relay-policy.v1".to_string(),
            status: "normative".to_string(),
            topology: RelayTopologySpec {
                sovereign_source: "forgejo".to_string(),
                public_projection: "github".to_string(),
                relationship: "relay".to_string(),
            },
            action: RelayActionSpec {
                target: "github".to_string(),
                credential: None,
            },
        }
    }

    #[test]
    fn passes_when_all_mandatory_gates_have_successful_proofs() {
        let contract = dummy_contract();
        let candidate = ReleaseCandidate {
            commit_sha: "abc123456789".to_string(),
            branch: "main".to_string(),
            proposed_version: ReleaseVersion::new(0, 2, 1, None),
            commit_messages: vec!["feat: initial".to_string()],
        };
        let proofs = vec![
            GateProofRecord {
                name: "cargo_check".to_string(),
                passed: true,
                detail: "ok".to_string(),
                observed_at_unix_s: 100,
            },
            GateProofRecord {
                name: "secret_lint".to_string(),
                passed: true,
                detail: "ok".to_string(),
                observed_at_unix_s: 100,
            },
        ];
        let prior = ReleaseVersion::new(0, 2, 0, None);

        let verdict = evaluate_release_candidate(&candidate, &contract, &proofs, Some(&prior));
        assert!(verdict.is_eligible());
    }

    #[test]
    fn blocks_when_mandatory_gate_fails() {
        let contract = dummy_contract();
        let candidate = ReleaseCandidate {
            commit_sha: "abc123456789".to_string(),
            branch: "main".to_string(),
            proposed_version: ReleaseVersion::new(0, 2, 1, None),
            commit_messages: vec![],
        };
        let proofs = vec![
            GateProofRecord {
                name: "cargo_check".to_string(),
                passed: false,
                detail: "syntax error".to_string(),
                observed_at_unix_s: 100,
            },
            GateProofRecord {
                name: "secret_lint".to_string(),
                passed: true,
                detail: "ok".to_string(),
                observed_at_unix_s: 100,
            },
        ];
        let verdict = evaluate_release_candidate(&candidate, &contract, &proofs, None);
        assert!(!verdict.is_eligible());
    }

    #[test]
    fn relay_authorized_when_eligible_and_tag() {
        let verdict = ReleaseEligibilityVerdict::Eligible {
            schema: "cynic.release_verdict.v1",
            version: ReleaseVersion::new(0, 2, 1, None),
            commit_sha: "abc123456789".to_string(),
            tag_name: "v0.2.1".to_string(),
            decided_at_unix_s: 100,
        };
        let relay_contract = dummy_relay_contract();
        let proof = evaluate_relay_eligibility(
            &verdict,
            &relay_contract,
            "github",
            "refs/tags/v0.2.1",
            false,
        );
        assert!(proof.is_authorized());
        match &proof {
            RelayDecisionProof::Authorized { action_kind, .. } => {
                assert_eq!(*action_kind, RelayActionKind::TagPush);
            }
            _ => panic!("expected authorized"),
        }
    }

    #[test]
    fn relay_rejected_when_blocked() {
        let verdict = ReleaseEligibilityVerdict::Blocked {
            schema: "cynic.release_verdict.v1",
            reasons: vec!["cargo_check failed".to_string()],
            decided_at_unix_s: 100,
        };
        let relay_contract = dummy_relay_contract();
        let proof =
            evaluate_relay_eligibility(&verdict, &relay_contract, "github", "main", false);
        assert!(!proof.is_authorized());
    }

    #[test]
    fn relay_uses_force_promote_when_divergent() {
        let verdict = ReleaseEligibilityVerdict::Eligible {
            schema: "cynic.release_verdict.v1",
            version: ReleaseVersion::new(0, 2, 1, None),
            commit_sha: "abc123456789".to_string(),
            tag_name: "v0.2.1".to_string(),
            decided_at_unix_s: 100,
        };
        let relay_contract = dummy_relay_contract();
        let proof =
            evaluate_relay_eligibility(&verdict, &relay_contract, "github", "main", true);
        assert!(proof.is_authorized());
        match &proof {
            RelayDecisionProof::Authorized { action_kind, .. } => {
                assert_eq!(*action_kind, RelayActionKind::MainForcedPromotion);
            }
            _ => panic!("expected authorized"),
        }
    }
}
