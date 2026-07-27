use super::{KYON_EXPERIMENT_PLAN_SCHEMA, KyonScenario, PlannedExperiment, admit_kyon_scenario};
use crate::AskesisError;

pub fn plan_experiment(
    scenario: &KyonScenario,
    experiment_id: &str,
) -> crate::Result<PlannedExperiment> {
    let admission = admit_kyon_scenario(scenario);
    if admission.status != "admitted" {
        return Err(AskesisError::KyonScenario(format!(
            "{} is not admitted: {}",
            scenario.id,
            admission.blockers.join(",")
        )));
    }

    let experiment = scenario
        .experiments
        .iter()
        .find(|experiment| experiment.id == experiment_id)
        .ok_or_else(|| {
            AskesisError::KyonScenario(format!(
                "{}: experiment {experiment_id} not found",
                scenario.id
            ))
        })?;

    Ok(PlannedExperiment {
        schema: KYON_EXPERIMENT_PLAN_SCHEMA.to_string(),
        scenario_id: scenario.id.clone(),
        experiment_id: experiment.id.clone(),
        protected_actor_id: scenario.protected_actor_id.clone(),
        desired_state: scenario.intent.statement.clone(),
        preflight: vec![
            "scenario_admitted".to_string(),
            "protected_actor_identity_observed".to_string(),
            "current_policy_version_observed".to_string(),
            "explicit_confirmation_required_before_apply".to_string(),
        ],
        baseline_observations: scenario
            .observations
            .iter()
            .map(|observation| observation.id.clone())
            .collect(),
        bounded_action: experiment.description.clone(),
        mutates_device_state: experiment.mutates_device_state,
        requires_explicit_confirmation: experiment.requires_confirmation,
        execution_authorized: false,
        stop_condition: experiment.stop_condition.clone(),
        read_back_observations: scenario.read_back.observations.clone(),
        sovereignty_outcomes: scenario.read_back.sovereignty_outcomes.clone(),
        forbidden_outcomes: experiment.forbidden_outcomes.clone(),
        residual_unknowns: scenario.read_back.residual_unknowns.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::kyon_scenarios::load_kyon_scenario_dataset;

    #[test]
    fn plan_is_inspectable_but_never_authorizes_execution() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("datasets")
            .join("kyon-scenarios-v0.jsonl");
        let dataset = load_kyon_scenario_dataset(&path).expect("load scenarios");
        let scenario = dataset
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "video-loop-sleep")
            .expect("video scenario");

        let plan = plan_experiment(scenario, "remove_notifications").expect("plan experiment");

        assert!(!plan.execution_authorized);
        assert!(plan.requires_explicit_confirmation);
        assert!(!plan.stop_condition.is_empty());
        assert!(!plan.read_back_observations.is_empty());
        assert!(!plan.residual_unknowns.is_empty());
    }
}
