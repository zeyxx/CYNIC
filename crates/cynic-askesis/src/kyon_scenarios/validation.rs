use std::collections::BTreeSet;

use super::types::{AuthorityKind, DataExposure, KYON_SCENARIO_SCHEMA, KyonScenario};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioAdmission {
    pub status: &'static str,
    pub blockers: Vec<String>,
}

pub fn admit_kyon_scenario(scenario: &KyonScenario) -> ScenarioAdmission {
    let mut blockers = Vec::new();

    if scenario.schema != KYON_SCENARIO_SCHEMA {
        blockers.push("scenario_schema_mismatch".to_string());
    }
    if scenario.id.trim().is_empty() || scenario.title.trim().is_empty() {
        blockers.push("scenario_identity_incomplete".to_string());
    }
    if scenario.harm_to_prevent.is_empty() {
        blockers.push("harm_to_prevent_missing".to_string());
    }

    let actor_ids = scenario
        .actors
        .iter()
        .map(|actor| actor.id.as_str())
        .collect::<BTreeSet<_>>();
    if actor_ids.len() != scenario.actors.len() {
        blockers.push("actor_ids_not_unique".to_string());
    }
    if !actor_ids.contains(scenario.protected_actor_id.as_str()) {
        blockers.push("protected_actor_missing".to_string());
    }
    if scenario.intent.owner_actor_id != scenario.protected_actor_id {
        blockers.push("intent_not_owned_by_protected_actor".to_string());
    }
    if !scenario.intent.revision_requires_explicit_confirmation {
        blockers.push("intertemporal_revision_confirmation_missing".to_string());
    }
    if scenario
        .actors
        .iter()
        .find(|actor| actor.id == scenario.protected_actor_id)
        .is_none_or(|actor| {
            !matches!(
                actor.authority,
                AuthorityKind::ProtectedPerson | AuthorityKind::Child
            )
        })
    {
        blockers.push("protected_actor_authority_invalid".to_string());
    }

    if scenario.observations.is_empty() {
        blockers.push("observation_surface_missing".to_string());
    }
    if scenario.hypotheses.len() < 2 {
        blockers.push("competing_hypotheses_required".to_string());
    }
    for hypothesis in &scenario.hypotheses {
        if hypothesis.statement.trim().is_empty()
            || hypothesis.disconfirming_observation.trim().is_empty()
        {
            blockers.push(format!("hypothesis_not_testable:{}", hypothesis.id));
        }
    }

    if scenario.experiments.is_empty() {
        blockers.push("protective_experiment_missing".to_string());
    }
    for experiment in &scenario.experiments {
        if !experiment.reversible {
            blockers.push(format!(
                "initial_experiment_not_reversible:{}",
                experiment.id
            ));
        }
        if !experiment.requires_confirmation {
            blockers.push(format!(
                "protective_experiment_confirmation_missing:{}",
                experiment.id
            ));
        }
        if experiment.stop_condition.trim().is_empty() {
            blockers.push(format!(
                "experiment_stop_condition_missing:{}",
                experiment.id
            ));
        }
    }

    if scenario.read_back.observations.is_empty() {
        blockers.push("read_back_observation_missing".to_string());
    }
    if scenario.read_back.sovereignty_outcomes.is_empty() {
        blockers.push("sovereignty_outcome_missing".to_string());
    }
    if scenario.read_back.residual_unknowns.is_empty() {
        blockers.push("residual_unknowns_missing".to_string());
    }
    if scenario.federation_enabled_by_default {
        blockers.push("federation_must_be_opt_in".to_string());
    }
    if scenario.raw_data_federation_allowed {
        blockers.push("raw_data_federation_forbidden_in_scenario_baseline".to_string());
    }
    if scenario.observations.iter().any(|observation| {
        observation.exposure == DataExposure::LocalRaw && observation.retention.trim().is_empty()
    }) {
        blockers.push("local_raw_retention_policy_missing".to_string());
    }

    ScenarioAdmission {
        status: if blockers.is_empty() {
            "admitted"
        } else {
            "blocked"
        },
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kyon_scenarios::{
        Actor, HumanIntent, ObservationSpec, ProtectiveExperiment, ReadBackSpec, ScenarioHypothesis,
    };
    use crate::kyon_scenarios::{EpistemicStatus, KyonScenario};

    fn valid_scenario() -> KyonScenario {
        KyonScenario {
            schema: KYON_SCENARIO_SCHEMA.to_string(),
            id: "choice-loop".to_string(),
            title: "Choice loop".to_string(),
            protected_actor_id: "alice".to_string(),
            actors: vec![Actor {
                id: "alice".to_string(),
                authority: AuthorityKind::ProtectedPerson,
                capabilities: vec!["choose".to_string()],
            }],
            harm_to_prevent: vec!["loss_of_choice".to_string()],
            intent: HumanIntent {
                owner_actor_id: "alice".to_string(),
                statement: "retain choice".to_string(),
                revision_requires_explicit_confirmation: true,
            },
            observations: vec![ObservationSpec {
                id: "event".to_string(),
                kind: "device_event".to_string(),
                epistemic_status: EpistemicStatus::Observed,
                exposure: DataExposure::LocalDerived,
                retention: "session".to_string(),
            }],
            hypotheses: vec![
                ScenarioHypothesis {
                    id: "a".to_string(),
                    level: "behavioral".to_string(),
                    statement: "cue driven".to_string(),
                    disconfirming_observation: "removing cue has no effect".to_string(),
                },
                ScenarioHypothesis {
                    id: "b".to_string(),
                    level: "affective".to_string(),
                    statement: "state regulation".to_string(),
                    disconfirming_observation: "state does not predict action".to_string(),
                },
            ],
            experiments: vec![ProtectiveExperiment {
                id: "pause".to_string(),
                description: "optional pause".to_string(),
                mutates_device_state: true,
                reversible: true,
                requires_confirmation: true,
                stop_condition: "alice stops".to_string(),
                forbidden_outcomes: vec!["hidden_block".to_string()],
            }],
            read_back: ReadBackSpec {
                observations: vec!["choice_after_pause".to_string()],
                sovereignty_outcomes: vec!["alice_reports_more_choice".to_string()],
                residual_unknowns: vec!["causal_mechanism".to_string()],
            },
            federation_enabled_by_default: false,
            raw_data_federation_allowed: false,
        }
    }

    #[test]
    fn admits_plural_reversible_opt_in_scenario() {
        assert_eq!(admit_kyon_scenario(&valid_scenario()).status, "admitted");
    }

    #[test]
    fn blocks_single_explanation_and_implicit_federation() {
        let mut scenario = valid_scenario();
        scenario.hypotheses.truncate(1);
        scenario.federation_enabled_by_default = true;

        let admission = admit_kyon_scenario(&scenario);

        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"competing_hypotheses_required".to_string())
        );
        assert!(
            admission
                .blockers
                .contains(&"federation_must_be_opt_in".to_string())
        );
    }
}
