use cynic_host::runtime_harness::{RuntimeHarnessContext, RuntimeHarnessTargetAttestation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HarnessTargetAttestationRequest {
    pub(crate) target_id: String,
    pub(crate) provider: String,
    pub(crate) transport: String,
    pub(crate) transport_target: String,
    pub(crate) expected_network_boundary: String,
}

pub(crate) trait HarnessTargetAttestationPort {
    fn attest_target(
        &self,
        context: &RuntimeHarnessContext,
        request: &HarnessTargetAttestationRequest,
    ) -> Result<RuntimeHarnessTargetAttestation, String>;
}
