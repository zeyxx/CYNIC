use cynic_protocol::EvidenceEnvelope;

pub trait ContentAddressedEvidenceStorePort: Send + Sync {
    fn persist(&self, envelope: &EvidenceEnvelope) -> Result<String, String>;
<<<<<<< HEAD

    fn read(&self, _digest: &str) -> Result<Option<EvidenceEnvelope>, String> {
        Err("evidence_read_not_supported".to_string())
    }
=======
>>>>>>> github/main
}
