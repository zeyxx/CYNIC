use std::collections::BTreeSet;

use super::{
    ExperimentOutcomeAssessment, ExperimentOutcomeEvidence, KYON_EXPERIMENT_ASSESSMENT_SCHEMA,
    PlannedExperiment,
};

pub fn assess_experiment_outcome(
    plan: &PlannedExperiment,
    evidence: &ExperimentOutcomeEvidence,
) -> ExperimentOutcomeAssessment {
    let mut violations = Vec::new();
    if evidence.scenario_id != plan.scenario_id {
        violations.push("scenario_identity_mismatch".to_string());
    }
    if evidence.experiment_id != plan.experiment_id {
        violations.push("experiment_identity_mismatch".to_string());
    }
    if plan.requires_explicit_confirmation && !evidence.confirmation_observed {
        violations.push("explicit_confirmation_not_observed".to_string());
    }
    if evidence.action_continued_after_stop {
        violations.push("action_continued_after_stop_condition".to_string());
    }

    let observed = evidence
        .read_back_observations
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let sovereignty = evidence
        .sovereignty_outcomes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let forbidden = evidence
        .forbidden_outcomes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let missing_observations = missing(&plan.read_back_observations, &observed);
    let missing_sovereignty_outcomes = missing(&plan.sovereignty_outcomes, &sovereignty);

    for forbidden_outcome in &plan.forbidden_outcomes {
        if forbidden.contains(forbidden_outcome.as_str()) {
            violations.push(format!("forbidden_outcome_observed:{forbidden_outcome}"));
        }
    }
    if evidence.evidence_refs.is_empty() {
        violations.push("read_back_evidence_missing".to_string());
    }

    let status = if !violations.is_empty() {
        "violated"
    } else if missing_observations.is_empty() && missing_sovereignty_outcomes.is_empty() {
        "read_back_complete"
    } else {
        "inconclusive"
    };
    let sovereignty_status = if violations.iter().any(|violation| {
        violation == "explicit_confirmation_not_observed"
            || violation == "action_continued_after_stop_condition"
            || violation.starts_with("forbidden_outcome_observed:")
    }) {
        "violated"
    } else if missing_sovereignty_outcomes.is_empty() {
        "observed"
    } else {
        "unknown"
    };

    ExperimentOutcomeAssessment {
        schema: KYON_EXPERIMENT_ASSESSMENT_SCHEMA.to_string(),
        scenario_id: plan.scenario_id.clone(),
        experiment_id: plan.experiment_id.clone(),
        status: status.to_string(),
        sovereignty_status: sovereignty_status.to_string(),
        causal_conclusion: "not_established_by_single_experiment".to_string(),
        missing_observations,
        missing_sovereignty_outcomes,
        violations,
        residual_unknowns: plan.residual_unknowns.clone(),
        evidence_refs: evidence.evidence_refs.clone(),
    }
}

fn missing(required: &[String], observed: &BTreeSet<&str>) -> Vec<String> {
    required
        .iter()
        .filter(|item| !observed.contains(item.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kyon_scenarios::KYON_EXPERIMENT_PLAN_SCHEMA;

    fn plan() -> PlannedExperiment {
        PlannedExperiment {
            schema: KYON_EXPERIMENT_PLAN_SCHEMA.to_string(),
            scenario_id: "video".to_string(),
            experiment_id: "pause".to_string(),
            protected_actor_id: "alice".to_string(),
            desired_state: "choose".to_string(),
            preflight: vec!["confirm".to_string()],
            baseline_observations: vec!["baseline".to_string()],
            bounded_action: "optional pause".to_string(),
            mutates_device_state: true,
            requires_explicit_confirmation: true,
            execution_authorized: false,
            stop_condition: "alice stops".to_string(),
            read_back_observations: vec!["choice".to_string()],
            sovereignty_outcomes: vec!["freedom".to_string()],
            forbidden_outcomes: vec!["hidden_block".to_string()],
            residual_unknowns: vec!["cause".to_string()],
        }
    }

    fn evidence() -> ExperimentOutcomeEvidence {
        ExperimentOutcomeEvidence {
            scenario_id: "video".to_string(),
            experiment_id: "pause".to_string(),
            confirmation_observed: true,
            stop_condition_observed: false,
            action_continued_after_stop: false,
            read_back_observations: vec!["choice".to_string()],
            sovereignty_outcomes: vec!["freedom".to_string()],
            forbidden_outcomes: Vec::new(),
            evidence_refs: vec!["sha256:proof".to_string()],
        }
    }

    #[test]
    fn complete_read_back_does_not_claim_causality() {
        let assessment = assess_experiment_outcome(&plan(), &evidence());

        assert_eq!(assessment.status, "read_back_complete");
        assert_eq!(
            assessment.causal_conclusion,
            "not_established_by_single_experiment"
        );
    }

    #[test]
    fn missing_confirmation_is_a_sovereignty_violation() {
        let mut evidence = evidence();
        evidence.confirmation_observed = false;

        let assessment = assess_experiment_outcome(&plan(), &evidence);

        assert_eq!(assessment.status, "violated");
        assert_eq!(assessment.sovereignty_status, "violated");
    }

    #[test]
    fn missing_read_back_remains_inconclusive() {
        let mut evidence = evidence();
        evidence.read_back_observations.clear();
        evidence.sovereignty_outcomes.clear();

        let assessment = assess_experiment_outcome(&plan(), &evidence);

        assert_eq!(assessment.status, "inconclusive");
        assert_eq!(assessment.sovereignty_status, "unknown");
    }

    #[test]
    fn continuing_after_stop_is_a_sovereignty_violation() {
        let mut evidence = evidence();
        evidence.stop_condition_observed = true;
        evidence.action_continued_after_stop = true;

        let assessment = assess_experiment_outcome(&plan(), &evidence);

        assert_eq!(assessment.status, "violated");
        assert_eq!(assessment.sovereignty_status, "violated");
        assert!(
            assessment
                .violations
                .contains(&"action_continued_after_stop_condition".to_string())
        );
    }
}
