use cynic_protocol::EvidenceEnvelope;

pub trait ContentAddressedEvidenceStorePort: Send + Sync {
    fn persist(&self, envelope: &EvidenceEnvelope) -> Result<String, String>;
}
