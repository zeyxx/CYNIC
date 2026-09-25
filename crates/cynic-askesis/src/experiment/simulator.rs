use crate::experiment::types::{ReconciliationAction, ServiceObservation, ServiceScenario};

#[derive(Debug)]
pub struct ServiceSimulator {
    scenario: ServiceScenario,
    current_state: crate::experiment::types::ServiceState,
}

impl ServiceSimulator {
    pub fn new(scenario: ServiceScenario) -> Self {
        let current_state = scenario.initial_state;
        Self {
            scenario,
            current_state,
        }
    }

    pub fn execute(
        &mut self,
        action: &ReconciliationAction,
    ) -> (Option<ServiceObservation>, String) {
        let service = match action {
            ReconciliationAction::ObserveService { service }
            | ReconciliationAction::RestartService { service, .. }
            | ReconciliationAction::ReadBackService { service } => service,
        };
        if service != &self.scenario.desired_service {
            return (None, "rejected_wrong_target".into());
        }
        match action {
            ReconciliationAction::ObserveService { service } => (
                Some(self.observation(service, "simulated_service_probe")),
                "observed".into(),
            ),
            ReconciliationAction::RestartService { consent: false, .. } => {
                (None, "rejected_missing_consent".into())
            }
            ReconciliationAction::RestartService { .. } => {
                if self.scenario.restart_succeeds {
                    self.current_state = crate::experiment::types::ServiceState::Running;
                    (None, "applied_pending_read_back".into())
                } else {
                    (None, "apply_failed".into())
                }
            }
            ReconciliationAction::ReadBackService { service } => (
                Some(self.observation(service, "simulated_post_action_probe")),
                "read_back".into(),
            ),
        }
    }

    fn observation(&self, service: &str, source: &str) -> ServiceObservation {
        ServiceObservation {
            service: service.into(),
            state: self.current_state,
            source: source.into(),
        }
    }
}
