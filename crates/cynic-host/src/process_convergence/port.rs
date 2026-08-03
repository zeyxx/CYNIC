use cynic_protocol::ServiceSupervisorObservation;

use crate::domain::services::ServicePolicy;

pub trait ProcessConvergenceObservationPort: Send + Sync {
    fn producer_id(&self) -> &str;

    fn observe(
        &self,
        subject_id: &str,
        policy: &ServicePolicy,
    ) -> Result<ServiceSupervisorObservation, String>;
}
