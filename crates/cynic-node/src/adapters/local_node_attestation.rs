use std::path::Path;

use cynic_protocol::NodeAttestation;

use crate::ports::node_attestation_port::NodeAttestationPort;

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalNodeAttestation;

impl NodeAttestationPort for LocalNodeAttestation {
    fn read(&self, source: &Path) -> Result<NodeAttestation, String> {
        let bytes =
            std::fs::read(source).map_err(|error| format!("read node attestation: {error}"))?;
        serde_json::from_slice(&bytes).map_err(|error| format!("decode node attestation: {error}"))
    }
}
