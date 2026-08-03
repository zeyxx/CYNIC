use std::path::PathBuf;

use cynic_host::{
    runtime_seed::{
        RuntimeSeedApplyReport, SshRuntimeSeedSignatureVerifier, apply_runtime_seed,
        persist_runtime_seed_evidence,
    },
    update_lifecycle::LocalArgvRunner,
};
use cynic_protocol::RuntimeSeedRequest;

use crate::ports::runtime_seed_actuation_port::RuntimeSeedActuationPort;

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalRuntimeSeedActuation;

impl RuntimeSeedActuationPort for LocalRuntimeSeedActuation {
    fn apply(&self, request: &RuntimeSeedRequest) -> RuntimeSeedApplyReport {
        apply_runtime_seed(request, &SshRuntimeSeedSignatureVerifier, &LocalArgvRunner)
    }

    fn persist(
        &self,
        request: &RuntimeSeedRequest,
        report: &RuntimeSeedApplyReport,
    ) -> Result<PathBuf, String> {
        persist_runtime_seed_evidence(std::path::Path::new(&request.install_root), report)
    }
}
