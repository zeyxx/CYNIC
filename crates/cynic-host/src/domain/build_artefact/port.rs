use cynic_build::build_artefact::BuildArtefactReport;

pub trait BuildArtefactObservationPort {
    fn observe_build_artefacts(&self) -> Result<BuildArtefactReport, String>;
    /// Fast path for consumers that need classification (orphan/aged) but not
    /// sizes: skips recursive `du` and network `ls-remote`. Sizes read 0 and
    /// the report names the skips in `residual_unknowns`. Defaults to the deep
    /// observation so remote adapters inherit it unchanged.
    fn observe_build_artefacts_shallow(&self) -> Result<BuildArtefactReport, String> {
        self.observe_build_artefacts()
    }
    fn plan_gc(
        &self,
        report: &BuildArtefactReport,
    ) -> cynic_build::build_artefact::BuildArtefactGcPlan;
}

pub trait BuildArtefactGcExecutionPort {
    fn execute_gc(
        &self,
        plan: &cynic_build::build_artefact::BuildArtefactGcPlan,
    ) -> cynic_build::build_artefact::BuildArtefactGcResult;
}
