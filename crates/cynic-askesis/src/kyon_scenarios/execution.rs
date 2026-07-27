use std::collections::BTreeSet;

use super::{
    ActionReceipt, BoundedActionCommand, ExecutionDecision, ExplicitConfirmation,
    KYON_ACTION_COMMAND_SCHEMA, KYON_EXECUTION_DECISION_SCHEMA, KYON_PREFLIGHT_OBSERVATION_SCHEMA,
    KyonActionPort, KyonObservationPort, PlannedExperiment, PreflightObservation,
};

pub fn observe_preflight<P: KyonObservationPort>(
    port: &P,
    plan: &PlannedExperiment,
) -> Result<PreflightObservation, P::Error> {
    port.observe_preflight(plan)
}

pub fn decide_execution(
    plan: &PlannedExperiment,
    observation: &PreflightObservation,
    confirmation: Option<&ExplicitConfirmation>,
) -> ExecutionDecision {
    let mut blockers = Vec::new();

    if observation.schema != KYON_PREFLIGHT_OBSERVATION_SCHEMA {
        blockers.push("preflight_schema_mismatch".to_string());
    }
    if observation.scenario_id != plan.scenario_id
        || observation.experiment_id != plan.experiment_id
    {
        blockers.push("preflight_identity_mismatch".to_string());
    }
    if observation.protected_actor_id != plan.protected_actor_id
        || !observation.protected_actor_identity_observed
    {
        blockers.push("protected_actor_identity_not_observed".to_string());
    }

    let observed = observation
        .baseline_observations
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for required in &plan.baseline_observations {
        if !observed.contains(required.as_str()) {
            blockers.push(format!("baseline_observation_missing:{required}"));
        }
    }
    if observation.evidence_refs.is_empty() {
        blockers.push("preflight_evidence_missing".to_string());
    }

    let policy_version = observation.policy_version.as_deref();
    if policy_version.is_none_or(str::is_empty) {
        blockers.push("policy_version_not_observed".to_string());
    }

    let confirmation = confirmation.filter(|confirmation| {
        let identity_matches = confirmation.scenario_id == plan.scenario_id
            && confirmation.experiment_id == plan.experiment_id
            && confirmation.actor_id == plan.protected_actor_id;
        if !identity_matches {
            blockers.push("confirmation_identity_mismatch".to_string());
        }
        let policy_matches = policy_version == Some(confirmation.policy_version.as_str());
        if !policy_matches {
            blockers.push("confirmation_policy_version_mismatch".to_string());
        }
        if confirmation.confirmation_ref.trim().is_empty() {
            blockers.push("confirmation_evidence_missing".to_string());
        }
        identity_matches && policy_matches && !confirmation.confirmation_ref.trim().is_empty()
    });

    if plan.requires_explicit_confirmation && confirmation.is_none() {
        blockers.push("explicit_confirmation_missing".to_string());
    }

    let command = if blockers.is_empty() {
        confirmation.map(|confirmation| BoundedActionCommand {
            schema: KYON_ACTION_COMMAND_SCHEMA.to_string(),
            scenario_id: plan.scenario_id.clone(),
            experiment_id: plan.experiment_id.clone(),
            protected_actor_id: plan.protected_actor_id.clone(),
            action: plan.bounded_action.clone(),
            mutates_device_state: plan.mutates_device_state,
            expected_policy_version: confirmation.policy_version.clone(),
            confirmation_ref: confirmation.confirmation_ref.clone(),
            stop_condition: plan.stop_condition.clone(),
            forbidden_outcomes: plan.forbidden_outcomes.clone(),
        })
    } else {
        None
    };

    ExecutionDecision {
        schema: KYON_EXECUTION_DECISION_SCHEMA.to_string(),
        scenario_id: plan.scenario_id.clone(),
        experiment_id: plan.experiment_id.clone(),
        status: if command.is_some() {
            "ready"
        } else {
            "blocked"
        }
        .to_string(),
        blockers,
        command,
        evidence_refs: observation.evidence_refs.clone(),
        residual_unknowns: observation.residual_unknowns.clone(),
    }
}

pub fn apply_authorized_action<P: KyonActionPort>(
    port: &P,
    decision: &ExecutionDecision,
) -> Result<Option<ActionReceipt>, P::Error> {
    match decision.command.as_ref() {
        Some(command) if decision.status == "ready" && decision.blockers.is_empty() => {
            port.apply_bounded_action(command).map(Some)
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::kyon_scenarios::KYON_EXPERIMENT_PLAN_SCHEMA;

    fn plan() -> PlannedExperiment {
        PlannedExperiment {
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
        }
    }

    fn observation() -> PreflightObservation {
        PreflightObservation {
            schema: KYON_PREFLIGHT_OBSERVATION_SCHEMA.to_string(),
            scenario_id: "scenario".to_string(),
            experiment_id: "experiment".to_string(),
            protected_actor_id: "alice".to_string(),
            protected_actor_identity_observed: true,
            policy_version: Some("policy-v1".to_string()),
            baseline_observations: vec!["baseline".to_string()],
            evidence_refs: vec!["sha256:preflight".to_string()],
            residual_unknowns: vec!["off_device_context".to_string()],
        }
    }

    fn confirmation() -> ExplicitConfirmation {
        ExplicitConfirmation {
            scenario_id: "scenario".to_string(),
            experiment_id: "experiment".to_string(),
            actor_id: "alice".to_string(),
            policy_version: "policy-v1".to_string(),
            confirmation_ref: "local-consent:1".to_string(),
        }
    }

    #[test]
    fn matching_observation_and_confirmation_produce_a_bounded_command() {
        let decision = decide_execution(&plan(), &observation(), Some(&confirmation()));

        assert_eq!(decision.status, "ready");
        let command = decision.command.expect("bounded command");
        assert_eq!(command.expected_policy_version, "policy-v1");
        assert_eq!(command.confirmation_ref, "local-consent:1");
        assert!(!command.mutates_device_state);
    }

    #[test]
    fn stale_confirmation_is_blocked() {
        let mut confirmation = confirmation();
        confirmation.policy_version = "policy-v0".to_string();

        let decision = decide_execution(&plan(), &observation(), Some(&confirmation));

        assert_eq!(decision.status, "blocked");
        assert!(decision.command.is_none());
        assert!(
            decision
                .blockers
                .contains(&"confirmation_policy_version_mismatch".to_string())
        );
    }

    struct CountingActionPort {
        calls: Cell<usize>,
    }

    impl KyonActionPort for CountingActionPort {
        type Error = ();

        fn apply_bounded_action(
            &self,
            command: &BoundedActionCommand,
        ) -> Result<ActionReceipt, Self::Error> {
            self.calls.set(self.calls.get() + 1);
            Ok(ActionReceipt {
                scenario_id: command.scenario_id.clone(),
                experiment_id: command.experiment_id.clone(),
                status: "applied".to_string(),
                action_evidence_refs: vec!["sha256:action".to_string()],
                residual_unknowns: Vec::new(),
            })
        }
    }

    #[test]
    fn blocked_decision_never_reaches_action_port() {
        let port = CountingActionPort {
            calls: Cell::new(0),
        };
        let decision = decide_execution(&plan(), &observation(), None);

        let receipt = apply_authorized_action(&port, &decision).expect("port result");

        assert!(receipt.is_none());
        assert_eq!(port.calls.get(), 0);
    }
}
