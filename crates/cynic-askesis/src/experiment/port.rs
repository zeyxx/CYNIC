use crate::experiment::types::{ReconciliationAction, ReconciliationStepProof, ServiceObservation};

pub trait ReconciliationDecisionPort {
    fn name(&self) -> &str;
    fn next_action(
        &self,
        desired: &ServiceObservation,
        history: &[ReconciliationStepProof],
    ) -> Option<ReconciliationAction>;
}
