use cynic_protocol::{
    HarnessObservation,
    HarnessScenario, HarnessScenarioId,
};

pub fn observe_harness_scenarios(
    scenarios: &[HarnessScenario],
    observed_data: Option<&[HarnessObservation]>,
) -> Vec<HarnessObservation> {
    scenarios
        .iter()
        .map(|scenario| {
            if let Some(custom) = observed_data
                .and_then(|items| items.iter().find(|obs| obs.scenario_id == Some(scenario.id)))
            {
                return custom.clone();
            }

            // Default host baseline observation (non-mutating, reports explicit unknowns if substrate unavailable)
            match scenario.id {
                HarnessScenarioId::ClosedLabVm => HarnessObservation {
                    scenario_id: Some(HarnessScenarioId::ClosedLabVm),
                    target_observed: false,
                    target_identity: None,
                    service_state: None,
                    evidences: Vec::new(),
                    staging_proof_present: false,
                    staging_proof_completed: false,
                    unknowns: vec!["libvirt_guest_unobserved".to_string()],
                    blockers: Vec::new(),
                },
                HarnessScenarioId::LxcCloneConformance => HarnessObservation {
                    scenario_id: Some(HarnessScenarioId::LxcCloneConformance),
                    target_observed: false,
                    target_identity: None,
                    service_state: None,
                    evidences: Vec::new(),
                    staging_proof_present: false,
                    staging_proof_completed: false,
                    unknowns: vec!["provider_observation_unavailable".to_string()],
                    blockers: Vec::new(),
                },
                HarnessScenarioId::BuilderStagingObserve => HarnessObservation {
                    scenario_id: Some(HarnessScenarioId::BuilderStagingObserve),
                    target_observed: false,
                    target_identity: None,
                    service_state: None,
                    evidences: Vec::new(),
                    staging_proof_present: false,
                    staging_proof_completed: false,
                    unknowns: vec!["provider_observation_unavailable".to_string()],
                    blockers: Vec::new(),
                },
                HarnessScenarioId::BuilderCanaryApply => HarnessObservation {
                    scenario_id: Some(HarnessScenarioId::BuilderCanaryApply),
                    target_observed: false,
                    target_identity: None,
                    service_state: None,
                    evidences: Vec::new(),
                    staging_proof_present: false,
                    staging_proof_completed: false,
                    unknowns: Vec::new(),
                    blockers: Vec::new(),
                },
                HarnessScenarioId::FailureRecovery => HarnessObservation {
                    scenario_id: Some(HarnessScenarioId::FailureRecovery),
                    target_observed: false,
                    target_identity: None,
                    service_state: None,
                    evidences: Vec::new(),
                    staging_proof_present: false,
                    staging_proof_completed: false,
                    unknowns: vec!["fault_harness_unobserved".to_string()],
                    blockers: Vec::new(),
                },
                HarnessScenarioId::ProofReplay => HarnessObservation {
                    scenario_id: Some(HarnessScenarioId::ProofReplay),
                    target_observed: false,
                    target_identity: None,
                    service_state: None,
                    evidences: Vec::new(),
                    staging_proof_present: false,
                    staging_proof_completed: false,
                    unknowns: vec!["retained_proof_unobserved".to_string()],
                    blockers: Vec::new(),
                },
            }
        })
        .collect()
}
