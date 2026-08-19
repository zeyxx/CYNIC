use cynic_protocol::{
    reconcile_harness_scenario, HarnessAction, HarnessCellReport, HarnessEvidence,
    HarnessEvidenceKind, HarnessEvidenceStatus, HarnessObservation, HarnessRollback,
    HarnessScenario, HarnessScenarioId, HarnessStatus,
};

pub fn observe_scheduler_conformance(
    node_id: &str,
    has_staging_proof: bool,
    active_canary_completion: Option<(&str, HarnessEvidenceStatus)>,
) -> Vec<HarnessObservation> {
    vec![
        HarnessObservation {
            scenario_id: Some(HarnessScenarioId::BuilderStagingObserve),
            target_observed: true,
            target_identity: Some(node_id.to_string()),
            service_state: Some("active".to_string()),
            evidences: if has_staging_proof {
                vec![HarnessEvidence {
                    kind: HarnessEvidenceKind::Observation,
                    status: HarnessEvidenceStatus::Completed,
                    reference: format!("staging-proof:{node_id}"),
                    digest: None,
                    detail: None,
                }]
            } else {
                Vec::new()
            },
            staging_proof_present: has_staging_proof,
            staging_proof_completed: has_staging_proof,
            unknowns: if has_staging_proof {
                Vec::new()
            } else {
                vec!["staging_read_back_missing".to_string()]
            },
            blockers: Vec::new(),
        },
        HarnessObservation {
            scenario_id: Some(HarnessScenarioId::BuilderCanaryApply),
            target_observed: true,
            target_identity: Some(node_id.to_string()),
            service_state: Some("active".to_string()),
            evidences: active_canary_completion
                .map(|(reference, status)| {
                    vec![HarnessEvidence {
                        kind: HarnessEvidenceKind::Execution,
                        status,
                        reference: reference.to_string(),
                        digest: None,
                        detail: None,
                    }]
                })
                .unwrap_or_default(),
            staging_proof_present: has_staging_proof,
            staging_proof_completed: has_staging_proof,
            unknowns: Vec::new(),
            blockers: if has_staging_proof {
                Vec::new()
            } else {
                vec!["staging_proof_missing".to_string()]
            },
        },
    ]
}

pub fn plan_builder_staging(scenario: &HarnessScenario, staging_observed: bool) -> HarnessCellReport {
    let obs = HarnessObservation {
        scenario_id: Some(HarnessScenarioId::BuilderStagingObserve),
        target_observed: staging_observed,
        target_identity: if staging_observed {
            Some("cynic-builder".to_string())
        } else {
            None
        },
        service_state: if staging_observed {
            Some("active".to_string())
        } else {
            None
        },
        evidences: if staging_observed {
            vec![HarnessEvidence {
                kind: HarnessEvidenceKind::Observation,
                status: HarnessEvidenceStatus::Completed,
                reference: "evidence:staging:observed".to_string(),
                digest: None,
                detail: None,
            }]
        } else {
            Vec::new()
        },
        staging_proof_present: staging_observed,
        staging_proof_completed: staging_observed,
        unknowns: if staging_observed {
            Vec::new()
        } else {
            vec!["provider_observation_unavailable".to_string()]
        },
        blockers: Vec::new(),
    };
    reconcile_harness_scenario(scenario, &obs)
}

pub fn plan_builder_canary(scenario: &HarnessScenario, staging_completed: bool) -> HarnessCellReport {
    let obs = HarnessObservation {
        scenario_id: Some(HarnessScenarioId::BuilderCanaryApply),
        target_observed: true,
        staging_proof_present: staging_completed,
        staging_proof_completed: staging_completed,
        unknowns: Vec::new(),
        blockers: if staging_completed {
            Vec::new()
        } else {
            vec!["staging_proof_missing".to_string()]
        },
        evidences: Vec::new(),
        ..Default::default()
    };
    reconcile_harness_scenario(scenario, &obs)
}

pub fn reduce_canary_completion(
    scenario: &HarnessScenario,
    evidence_status: HarnessEvidenceStatus,
    reference: &str,
) -> HarnessCellReport {
    let obs = HarnessObservation {
        scenario_id: Some(HarnessScenarioId::BuilderCanaryApply),
        target_observed: true,
        staging_proof_present: true,
        staging_proof_completed: true,
        evidences: vec![HarnessEvidence {
            kind: HarnessEvidenceKind::Execution,
            status: evidence_status,
            reference: reference.to_string(),
            digest: None,
            detail: None,
        }],
        unknowns: Vec::new(),
        blockers: Vec::new(),
        ..Default::default()
    };
    reconcile_harness_scenario(scenario, &obs)
}

pub fn replay_proof(
    scenario: &HarnessScenario,
    expected_digest: &str,
    observed_digest: &str,
) -> HarnessCellReport {
    let matches = expected_digest == observed_digest;
    let obs = HarnessObservation {
        scenario_id: Some(HarnessScenarioId::ProofReplay),
        target_observed: true,
        evidences: vec![HarnessEvidence {
            kind: HarnessEvidenceKind::Replay,
            status: if matches {
                HarnessEvidenceStatus::Completed
            } else {
                HarnessEvidenceStatus::Stale
            },
            reference: format!("replay:digest:{observed_digest}"),
            digest: Some(observed_digest.to_string()),
            detail: None,
        }],
        unknowns: Vec::new(),
        blockers: Vec::new(),
        ..Default::default()
    };
    reconcile_harness_scenario(scenario, &obs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_observes_without_dispatching() {
        let scenario = HarnessScenario::builder_staging_observe();
        let report = plan_builder_staging(&scenario, true);
        assert_eq!(report.action, Some(HarnessAction::Observe));
        assert_eq!(report.status, HarnessStatus::Completed);
    }

    #[test]
    fn canary_is_blocked_without_staging_proof() {
        let scenario = HarnessScenario::builder_canary_apply();
        let report = plan_builder_canary(&scenario, false);
        assert_eq!(report.status, HarnessStatus::Blocked);
        assert!(report.blockers.contains(&"staging_proof_missing".to_string()));
    }

    #[test]
    fn cancelled_canary_reduces_to_cancelled_status() {
        let scenario = HarnessScenario::builder_canary_apply();
        let report = reduce_canary_completion(
            &scenario,
            HarnessEvidenceStatus::Cancelled,
            "cancellation:workload-100",
        );
        assert_eq!(report.status, HarnessStatus::Cancelled);
        assert_eq!(report.evidence_refs, vec!["cancellation:workload-100".to_string()]);
    }

    #[test]
    fn replay_detects_drift_and_yields_stale_status() {
        let scenario = HarnessScenario::proof_replay();
        let report = replay_proof(&scenario, "sha256:abc123", "sha256:def456");
        assert_eq!(report.status, HarnessStatus::Stale);
    }
}
