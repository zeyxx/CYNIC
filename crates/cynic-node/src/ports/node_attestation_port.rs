use std::path::Path;

use cynic_protocol::NodeAttestation;

/// Portable boundary for measured-image/TPM evidence. The transport and
/// provider-specific quote format stay outside the enrollment protocol.
pub trait NodeAttestationPort: Send + Sync {
    fn read(&self, source: &Path) -> Result<NodeAttestation, String>;
}
