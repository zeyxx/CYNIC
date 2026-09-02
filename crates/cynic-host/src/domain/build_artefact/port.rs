use cynic_protocol::build_artefact::BuildArtefactReport;

pub trait BuildArtefactObservationPort {
    fn observe_build_artefacts(&self) -> Result<BuildArtefactReport, String>;
    fn plan_gc(
        &self,
        report: &BuildArtefactReport,
    ) -> cynic_protocol::build_artefact::BuildArtefactGcPlan;
}

pub trait BuildArtefactGcExecutionPort {
    fn execute_gc(
        &self,
        plan: &cynic_protocol::build_artefact::BuildArtefactGcPlan,
    ) -> cynic_protocol::build_artefact::BuildArtefactGcResult;
}
