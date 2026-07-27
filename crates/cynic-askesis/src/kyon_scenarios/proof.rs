use super::{
    ActionReceipt, ExecutionDecision, ExperimentOutcomeEvidence, ExperimentProofBundle,
    KYON_EXPERIMENT_PROOF_SCHEMA, KYON_READ_BACK_REQUEST_SCHEMA, KyonReadBackPort,
    PlannedExperiment, ReadBackRequest, assess_experiment_outcome,
};
use crate::AskesisError;

pub fn build_read_back_request(
    plan: &PlannedExperiment,
    decision: &ExecutionDecision,
    receipt: &ActionReceipt,
) -> crate::Result<ReadBackRequest> {
    let command = decision.command.as_ref().ok_or_else(|| {
        AskesisError::KyonScenario("read-back requires an admitted action command".to_string())
    })?;
    if decision.status != "ready" || !decision.blockers.is_empty() {
        return Err(AskesisError::KyonScenario(
            "read-back cannot follow a blocked execution decision".to_string(),
        ));
    }
    if command.scenario_id != plan.scenario_id
        || command.experiment_id != plan.experiment_id
        || receipt.scenario_id != plan.scenario_id
        || receipt.experiment_id != plan.experiment_id
    {
        return Err(AskesisError::KyonScenario(
            "read-back identity chain mismatch".to_string(),
        ));
    }

    Ok(ReadBackRequest {
        schema: KYON_READ_BACK_REQUEST_SCHEMA.to_string(),
        scenario_id: plan.scenario_id.clone(),
        experiment_id: plan.experiment_id.clone(),
        protected_actor_id: plan.protected_actor_id.clone(),
        expected_policy_version: command.expected_policy_version.clone(),
        confirmation_ref: command.confirmation_ref.clone(),
        stop_condition: command.stop_condition.clone(),
        required_observations: plan.read_back_observations.clone(),
        required_sovereignty_outcomes: plan.sovereignty_outcomes.clone(),
        forbidden_outcomes: plan.forbidden_outcomes.clone(),
        action_evidence_refs: receipt.action_evidence_refs.clone(),
    })
}

pub fn observe_read_back<P: KyonReadBackPort>(
    port: &P,
    request: &ReadBackRequest,
) -> Result<ExperimentOutcomeEvidence, P::Error> {
    port.observe_read_back(request)
}

pub fn assemble_experiment_proof(
    plan: &PlannedExperiment,
    decision: &ExecutionDecision,
    receipt: &ActionReceipt,
    evidence: &ExperimentOutcomeEvidence,
) -> ExperimentProofBundle {
    let assessment = assess_experiment_outcome(plan, evidence);
    let mut blockers = Vec::new();

    if decision.scenario_id != plan.scenario_id || decision.experiment_id != plan.experiment_id {
        blockers.push("execution_decision_identity_mismatch".to_string());
    }
    if decision.evidence_refs.is_empty() {
        blockers.push("preflight_evidence_missing".to_string());
    }
    if decision.command.as_ref().is_some_and(|command| {
        command.scenario_id != plan.scenario_id
            || command.experiment_id != plan.experiment_id
            || command.protected_actor_id != plan.protected_actor_id
    }) {
        blockers.push("action_command_identity_mismatch".to_string());
    }
    if receipt.scenario_id != plan.scenario_id || receipt.experiment_id != plan.experiment_id {
        blockers.push("action_receipt_identity_mismatch".to_string());
    }
    if receipt.status != "applied" {
        blockers.push("action_not_proved_applied".to_string());
    }
    if receipt.action_evidence_refs.is_empty() {
        blockers.push("action_evidence_missing".to_string());
    }
    if decision.status != "ready" || !decision.blockers.is_empty() || decision.command.is_none() {
        blockers.push("execution_decision_not_ready".to_string());
    }
    blockers.extend(assessment.violations.iter().cloned());
    blockers.sort();
    blockers.dedup();

    let command = decision.command.as_ref();
    let status = if !blockers.is_empty() {
        "violated"
    } else if assessment.status == "read_back_complete" {
        "proved"
    } else {
        "inconclusive"
    };
    let mut residual_unknowns = decision.residual_unknowns.clone();
    residual_unknowns.extend(receipt.residual_unknowns.iter().cloned());
    residual_unknowns.extend(assessment.residual_unknowns.iter().cloned());
    residual_unknowns.sort();
    residual_unknowns.dedup();

    ExperimentProofBundle {
        schema: KYON_EXPERIMENT_PROOF_SCHEMA.to_string(),
        scenario_id: plan.scenario_id.clone(),
        experiment_id: plan.experiment_id.clone(),
        status: status.to_string(),
        policy_version: command.map(|command| command.expected_policy_version.clone()),
        confirmation_ref: command.map(|command| command.confirmation_ref.clone()),
        preflight_evidence_refs: decision.evidence_refs.clone(),
        action_evidence_refs: receipt.action_evidence_refs.clone(),
        read_back_evidence_refs: evidence.evidence_refs.clone(),
        assessment,
        blockers,
        residual_unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kyon_scenarios::{
        BoundedActionCommand, KYON_ACTION_COMMAND_SCHEMA, KYON_EXECUTION_DECISION_SCHEMA,
        KYON_EXPERIMENT_PLAN_SCHEMA,
    };

    fn fixtures() -> (
        PlannedExperiment,
        ExecutionDecision,
        ActionReceipt,
        ExperimentOutcomeEvidence,
    ) {
        let plan = PlannedExperiment {
            schema: KYON_EXPERIMENT_PLAN_SCHEMA.to_string(),
            scenario_id: "scenario".to_string(),
            experiment_id: "experiment".to_string(),
            protected_actor_id: "alice".to_string(),
            desired_state: "retain choice".to_string(),
            preflight: vec!["observe".to_string()],
            baseline_observations: vec!["baseline".to_string()],
            bounded_action: "offer pause".to_string(),
            mutates_device_state: false,
            requires_explicit_confirmation: true,
            execution_authorized: false,
            stop_condition: "alice stops".to_string(),
            read_back_observations: vec!["choice".to_string()],
            sovereignty_outcomes: vec!["freedom".to_string()],
            forbidden_outcomes: vec!["hidden_block".to_string()],
            residual_unknowns: vec!["causality".to_string()],
        };
        let command = BoundedActionCommand {
            schema: KYON_ACTION_COMMAND_SCHEMA.to_string(),
            scenario_id: "scenario".to_string(),
            experiment_id: "experiment".to_string(),
            protected_actor_id: "alice".to_string(),
            action: "offer pause".to_string(),
            mutates_device_state: false,
            expected_policy_version: "policy-v1".to_string(),
            confirmation_ref: "local-consent:1".to_string(),
            stop_condition: "alice stops".to_string(),
            forbidden_outcomes: vec!["hidden_block".to_string()],
        };
        let decision = ExecutionDecision {
            schema: KYON_EXECUTION_DECISION_SCHEMA.to_string(),
            scenario_id: "scenario".to_string(),
            experiment_id: "experiment".to_string(),
            status: "ready".to_string(),
            blockers: Vec::new(),
            command: Some(command),
            evidence_refs: vec!["sha256:preflight".to_string()],
            residual_unknowns: vec!["off_device_context".to_string()],
        };
        let receipt = ActionReceipt {
            scenario_id: "scenario".to_string(),
            experiment_id: "experiment".to_string(),
            status: "applied".to_string(),
            action_evidence_refs: vec!["sha256:action".to_string()],
            residual_unknowns: Vec::new(),
        };
        let evidence = ExperimentOutcomeEvidence {
            scenario_id: "scenario".to_string(),
            experiment_id: "experiment".to_string(),
            confirmation_observed: true,
            stop_condition_observed: false,
            action_continued_after_stop: false,
            read_back_observations: vec!["choice".to_string()],
            sovereignty_outcomes: vec!["freedom".to_string()],
            forbidden_outcomes: Vec::new(),
            evidence_refs: vec!["sha256:read-back".to_string()],
        };
        (plan, decision, receipt, evidence)
    }

    #[test]
    fn read_back_request_preserves_consent_and_action_chain() {
        let (plan, decision, receipt, _) = fixtures();

        let request =
            build_read_back_request(&plan, &decision, &receipt).expect("read-back request");

        assert_eq!(request.confirmation_ref, "local-consent:1");
        assert_eq!(request.action_evidence_refs, vec!["sha256:action"]);
        assert_eq!(request.required_sovereignty_outcomes, vec!["freedom"]);
    }

    #[test]
    fn complete_chain_produces_proof_without_claiming_causality() {
        let (plan, decision, receipt, evidence) = fixtures();

        let proof = assemble_experiment_proof(&plan, &decision, &receipt, &evidence);

        assert_eq!(proof.status, "proved");
        assert_eq!(
            proof.assessment.causal_conclusion,
            "not_established_by_single_experiment"
        );
        assert_eq!(proof.policy_version.as_deref(), Some("policy-v1"));
    }

    #[test]
    fn successful_claim_without_action_evidence_is_not_proof() {
        let (plan, decision, mut receipt, evidence) = fixtures();
        receipt.action_evidence_refs.clear();

        let proof = assemble_experiment_proof(&plan, &decision, &receipt, &evidence);

        assert_eq!(proof.status, "violated");
        assert!(
            proof
                .blockers
                .contains(&"action_evidence_missing".to_string())
        );
    }

    #[test]
    fn mismatched_decision_identity_cannot_be_reassembled_as_proof() {
        let (plan, mut decision, receipt, evidence) = fixtures();
        decision.scenario_id = "other-scenario".to_string();

        let proof = assemble_experiment_proof(&plan, &decision, &receipt, &evidence);

        assert_eq!(proof.status, "violated");
        assert!(
            proof
                .blockers
                .contains(&"execution_decision_identity_mismatch".to_string())
        );
    }
}
