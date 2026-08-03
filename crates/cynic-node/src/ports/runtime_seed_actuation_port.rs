use std::path::PathBuf;

use cynic_host::runtime_seed::RuntimeSeedApplyReport;
use cynic_protocol::RuntimeSeedRequest;

pub trait RuntimeSeedActuationPort {
    fn apply(&self, request: &RuntimeSeedRequest) -> RuntimeSeedApplyReport;

    fn persist(
        &self,
        request: &RuntimeSeedRequest,
        report: &RuntimeSeedApplyReport,
    ) -> Result<PathBuf, String>;
}
