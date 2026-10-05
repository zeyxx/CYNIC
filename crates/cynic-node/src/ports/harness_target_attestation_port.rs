use cynic_host::runtime_harness::{RuntimeHarnessContext, RuntimeHarnessTargetAttestation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessTargetAttestationRequest {
    pub target_id: String,
    pub provider: String,
    pub transport: String,
    pub transport_target: String,
    pub expected_network_boundary: String,
}

pub trait HarnessTargetAttestationPort {
    fn attest_target(
        &self,
        context: &RuntimeHarnessContext,
        request: &HarnessTargetAttestationRequest,
    ) -> Result<RuntimeHarnessTargetAttestation, String>;
}
