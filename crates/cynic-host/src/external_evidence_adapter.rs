//! External evidence adapter for cynic-host.
//!
//! WHY: implements ExternalEvidencePort by wrapping the content-addressed
//! evidence store. All I/O lives here (adapter layer, FCIS).
//! HOW: delegates persistence to ContentAddressedEvidenceStorePort.
//! NEGATIVE BOUNDARY: no domain logic, no admission — those live in
//! cynic-kernel::ports::external_evidence.

use cynic_kernel::ports::external_evidence::{
    ExternalEvidenceBatch, ExternalEvidenceIngestionResult, ExternalEvidencePort,
};
use cynic_protocol::EvidenceEnvelope;

/// Adapter that wraps a content-addressed evidence store.
pub struct LocalExternalEvidenceAdapter<S> {
    pub store: S,
}

impl<S> LocalExternalEvidenceAdapter<S>
where
    S: crate::ports::evidence_store::ContentAddressedEvidenceStorePort,
{
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

impl<S> ExternalEvidencePort for LocalExternalEvidenceAdapter<S>
where
    S: crate::ports::evidence_store::ContentAddressedEvidenceStorePort,
{
    fn ingest(&self, batch: &ExternalEvidenceBatch) -> ExternalEvidenceIngestionResult {
        // Build an EvidenceEnvelope from the batch
        let envelope = EvidenceEnvelope {
            schema: "cynic.kernel.external_evidence.v1".to_string(),
            digest: String::new(), // store will compute
            payload_digest: String::new(),
            subject: batch.subject.clone(),
            producer: batch.producer.clone(),
            observed_at_unix_ms: batch.observed_at_unix_ms,
            provenance: batch.provenance.clone(),
            lineage: None,
            payload: batch.payload.clone(),
        };

        match self.store.persist(&envelope) {
            Ok(digest) => ExternalEvidenceIngestionResult {
                project_id: batch.project_id.clone(),
                subject: batch.subject.clone(),
                digest,
                persisted: true,
                error: None,
            },
            Err(e) => ExternalEvidenceIngestionResult {
                project_id: batch.project_id.clone(),
                subject: batch.subject.clone(),
                digest: String::new(),
                persisted: false,
                error: Some(e),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::evidence_store::ContentAddressedEvidenceStorePort;

    struct MockStore;
    impl ContentAddressedEvidenceStorePort for MockStore {
        fn persist(&self, _envelope: &EvidenceEnvelope) -> Result<String, String> {
            Ok("mock-digest-123".to_string())
        }
    }

    #[test]
    fn ingest_persists() {
        let adapter = LocalExternalEvidenceAdapter::new(MockStore);
        let batch = ExternalEvidenceBatch {
            project_id: "aegisscan".to_string(),
            producer: "aegisscan-findings-v2".to_string(),
            subject: "pentest-run-1".to_string(),
            observed_at_unix_ms: 1000,
            provenance: vec![],
            payload: serde_json::json!({"finding": "test"}),
        };
        let result = adapter.ingest(&batch);
        assert!(result.persisted);
        assert_eq!(result.digest, "mock-digest-123");
    }
}