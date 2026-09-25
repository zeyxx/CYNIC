use crate::experiment::port::ReconciliationDecisionPort;
use crate::experiment::simulator::ServiceSimulator;
use crate::experiment::types::{
    ReconciliationAction, ReconciliationProof, ReconciliationStepProof, ServiceObservation,
    ServiceScenario,
};

pub fn run_reconciliation_episode<P: ReconciliationDecisionPort>(
    scenario: ServiceScenario,
    max_actions: u32,
    policy: &P,
) -> ReconciliationProof {
    let desired = ServiceObservation {
        service: scenario.desired_service.clone(),
        state: scenario.desired_state,
        source: "experiment_spec".into(),
    };
    let mut simulator = ServiceSimulator::new(scenario);
    let mut steps = Vec::new();
    let mut unknowns = Vec::new();
    let mut mutation_count = 0;
    let mut converged = false;

    for step in 1..=max_actions {
        let Some(action) = policy.next_action(&desired, &steps) else {
            unknowns.push("policy_produced_no_bounded_action".into());
            break;
        };
        let mutates = matches!(action, ReconciliationAction::RestartService { .. });
        mutation_count += u32::from(mutates);
        let (observation, outcome) = simulator.execute(&action);
        let is_read_back = matches!(action, ReconciliationAction::ReadBackService { .. });
        converged = is_read_back
            && observation.as_ref().is_some_and(|observed| {
                observed.service == desired.service && observed.state == desired.state
            });
        let failed = outcome.starts_with("rejected") || outcome == "apply_failed";
        steps.push(ReconciliationStepProof {
            step,
            action,
            mutates,
            observation,
            outcome,
        });
        if converged || failed {
            break;
        }
    }
    if !converged && unknowns.is_empty() {
        unknowns.push("convergence_not_proved_within_action_budget".into());
    }
    ReconciliationProof {
        schema: "cynic.askesis.reconciliation.proof.v1",
        status: if converged { "converged" } else { "blocked" },
        policy: policy.name().into(),
        desired_state: desired,
        max_actions,
        converged,
        mutation_count,
        steps,
        unknowns,
    }
}

#[derive(Debug, Default)]
pub struct ObserveReconcileReadBackPolicy;

impl ReconciliationDecisionPort for ObserveReconcileReadBackPolicy {
    fn name(&self) -> &str {
        "observe_reconcile_read_back.v1"
    }

    fn next_action(
        &self,
        desired: &ServiceObservation,
        history: &[ReconciliationStepProof],
    ) -> Option<ReconciliationAction> {
        match history.last() {
            None => Some(ReconciliationAction::ObserveService {
                service: desired.service.clone(),
            }),
            Some(last)
                if last
                    .observation
                    .as_ref()
                    .is_some_and(|observed| observed.state == desired.state) =>
            {
                Some(ReconciliationAction::ReadBackService {
                    service: desired.service.clone(),
                })
            }
            Some(last) if matches!(last.action, ReconciliationAction::RestartService { .. }) => {
                Some(ReconciliationAction::ReadBackService {
                    service: desired.service.clone(),
                })
            }
            Some(_) => Some(ReconciliationAction::RestartService {
                service: desired.service.clone(),
                consent: true,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment::types::ServiceState;

    fn stopped_service(restart_succeeds: bool) -> ServiceScenario {
        ServiceScenario {
            desired_service: "cynic-node".into(),
            desired_state: ServiceState::Running,
            initial_state: ServiceState::Stopped,
            restart_succeeds,
        }
    }

    #[test]
    fn proves_convergence_only_after_post_action_read_back() {
        let proof =
            run_reconciliation_episode(stopped_service(true), 3, &ObserveReconcileReadBackPolicy);
        assert!(proof.converged);
        assert_eq!(proof.mutation_count, 1);
        assert_eq!(
            proof.steps.last().map(|step| step.outcome.as_str()),
            Some("read_back")
        );
    }

    #[test]
    fn failed_apply_remains_blocked() {
        let proof =
            run_reconciliation_episode(stopped_service(false), 3, &ObserveReconcileReadBackPolicy);
        assert!(!proof.converged);
        assert_eq!(proof.status, "blocked");
        assert_eq!(
            proof.unknowns,
            vec!["convergence_not_proved_within_action_budget"]
        );
    }
}
