use std::path::Path;

use cynic_host::runtime_harness::RuntimeHarnessTargetAttestation;

pub(crate) trait HarnessTargetAttestationReaderPort {
    fn read_attestation(&self, path: &Path) -> Result<RuntimeHarnessTargetAttestation, String>;
}
