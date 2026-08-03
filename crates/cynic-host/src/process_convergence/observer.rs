use cynic_protocol::ServiceSupervisorObservation;

use super::port::ProcessConvergenceObservationPort;
use crate::domain::services::ServicePolicy;
use crate::service_supervisor_observation::observe_service_supervisor;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalProcessConvergenceObserver;

impl ProcessConvergenceObservationPort for LocalProcessConvergenceObserver {
    fn producer_id(&self) -> &str {
        "cynic-host:local-service-supervisor-observer"
    }

    fn observe(
        &self,
        subject_id: &str,
        policy: &ServicePolicy,
    ) -> Result<ServiceSupervisorObservation, String> {
        Ok(observe_service_supervisor(subject_id.to_string(), policy))
    }
}
