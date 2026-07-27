use super::{
    ActionReceipt, BoundedActionCommand, ExperimentOutcomeEvidence, PlannedExperiment,
    PreflightObservation, ReadBackRequest,
};

pub trait KyonObservationPort {
    type Error;

    fn observe_preflight(
        &self,
        plan: &PlannedExperiment,
    ) -> Result<PreflightObservation, Self::Error>;
}

pub trait KyonReadBackPort {
    type Error;

    fn observe_read_back(
        &self,
        request: &ReadBackRequest,
    ) -> Result<ExperimentOutcomeEvidence, Self::Error>;
}

pub trait KyonActionPort {
    type Error;

    fn apply_bounded_action(
        &self,
        command: &BoundedActionCommand,
    ) -> Result<ActionReceipt, Self::Error>;
}
