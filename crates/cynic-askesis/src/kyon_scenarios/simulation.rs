use std::cell::RefCell;

use super::{
    ActionReceipt, BoundedActionCommand, ExperimentOutcomeEvidence, KyonActionPort,
    KyonObservationPort, KyonReadBackPort, PlannedExperiment, PreflightObservation,
    ReadBackRequest,
};

#[derive(Debug)]
pub struct SimulatedKyonAdapter {
    preflight: PreflightObservation,
    receipt: ActionReceipt,
    outcome: ExperimentOutcomeEvidence,
    applied_commands: RefCell<Vec<BoundedActionCommand>>,
}

impl SimulatedKyonAdapter {
    pub fn new(
        preflight: PreflightObservation,
        receipt: ActionReceipt,
        outcome: ExperimentOutcomeEvidence,
    ) -> Self {
        Self {
            preflight,
            receipt,
            outcome,
            applied_commands: RefCell::new(Vec::new()),
        }
    }

    pub fn applied_commands(&self) -> Vec<BoundedActionCommand> {
        self.applied_commands.borrow().clone()
    }
}

impl KyonObservationPort for SimulatedKyonAdapter {
    type Error = String;

    fn observe_preflight(
        &self,
        plan: &PlannedExperiment,
    ) -> Result<PreflightObservation, Self::Error> {
        if self.preflight.scenario_id != plan.scenario_id
            || self.preflight.experiment_id != plan.experiment_id
        {
            return Err("simulated_preflight_identity_mismatch".to_string());
        }
        Ok(self.preflight.clone())
    }
}

impl KyonActionPort for SimulatedKyonAdapter {
    type Error = String;

    fn apply_bounded_action(
        &self,
        command: &BoundedActionCommand,
    ) -> Result<ActionReceipt, Self::Error> {
        if self.receipt.scenario_id != command.scenario_id
            || self.receipt.experiment_id != command.experiment_id
        {
            return Err("simulated_action_identity_mismatch".to_string());
        }
        self.applied_commands.borrow_mut().push(command.clone());
        Ok(self.receipt.clone())
    }
}

impl KyonReadBackPort for SimulatedKyonAdapter {
    type Error = String;

    fn observe_read_back(
        &self,
        request: &ReadBackRequest,
    ) -> Result<ExperimentOutcomeEvidence, Self::Error> {
        if self.outcome.scenario_id != request.scenario_id
            || self.outcome.experiment_id != request.experiment_id
        {
            return Err("simulated_read_back_identity_mismatch".to_string());
        }
        Ok(self.outcome.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::kyon_scenarios::{
        ExplicitConfirmation, KYON_PREFLIGHT_OBSERVATION_SCHEMA, apply_authorized_action,
        assemble_experiment_proof, build_read_back_request, decide_execution,
        load_kyon_scenario_dataset, observe_preflight, observe_read_back, plan_experiment,
    };

    #[test]
    fn alice_incoming_content_vertical_produces_local_sovereignty_proof() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("datasets")
            .join("kyon-scenarios-v0.jsonl");
        let dataset = load_kyon_scenario_dataset(&path).expect("load scenarios");
        let scenario = dataset
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "alice-unwanted-sexual-image")
            .expect("incoming content scenario");
        let plan = plan_experiment(scenario, "local_protected_preview").expect("experiment plan");
        let adapter = SimulatedKyonAdapter::new(
            PreflightObservation {
                schema: KYON_PREFLIGHT_OBSERVATION_SCHEMA.to_string(),
                scenario_id: scenario.id.clone(),
                experiment_id: "local_protected_preview".to_string(),
                protected_actor_id: "alice".to_string(),
                protected_actor_identity_observed: true,
                policy_version: Some("alice-policy-v1".to_string()),
                baseline_observations: plan.baseline_observations.clone(),
                evidence_refs: vec!["local:preflight-content-envelope".to_string()],
                residual_unknowns: vec!["sender_intent".to_string()],
            },
            ActionReceipt {
                scenario_id: scenario.id.clone(),
                experiment_id: "local_protected_preview".to_string(),
                status: "applied".to_string(),
                action_evidence_refs: vec!["local:protected-preview-rendered".to_string()],
                residual_unknowns: Vec::new(),
            },
            ExperimentOutcomeEvidence {
                scenario_id: scenario.id.clone(),
                experiment_id: "local_protected_preview".to_string(),
                confirmation_observed: true,
                stop_condition_observed: false,
                action_continued_after_stop: false,
                read_back_observations: plan.read_back_observations.clone(),
                sovereignty_outcomes: plan.sovereignty_outcomes.clone(),
                forbidden_outcomes: Vec::new(),
                evidence_refs: vec!["local:alice-choice-read-back".to_string()],
            },
        );

        let observed = observe_preflight(&adapter, &plan).expect("observe preflight");
        let confirmation = ExplicitConfirmation {
            scenario_id: scenario.id.clone(),
            experiment_id: "local_protected_preview".to_string(),
            actor_id: "alice".to_string(),
            policy_version: "alice-policy-v1".to_string(),
            confirmation_ref: "local:alice-opt-in-v1".to_string(),
        };
        let decision = decide_execution(&plan, &observed, Some(&confirmation));
        let receipt = apply_authorized_action(&adapter, &decision)
            .expect("apply result")
            .expect("action receipt");
        let request =
            build_read_back_request(&plan, &decision, &receipt).expect("read-back request");
        let outcome = observe_read_back(&adapter, &request).expect("read-back");
        let proof = assemble_experiment_proof(&plan, &decision, &receipt, &outcome);

        assert_eq!(proof.status, "proved");
        assert_eq!(adapter.applied_commands().len(), 1);
        assert!(
            proof
                .residual_unknowns
                .contains(&"sender_intent".to_string())
        );
        assert_eq!(
            proof.assessment.causal_conclusion,
            "not_established_by_single_experiment"
        );
    }
}
