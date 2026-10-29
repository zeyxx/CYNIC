//! External evidence adapter for cynic-node.

use cynic_protocol::evidence_store_port::ContentAddressedEvidenceStorePort;
use cynic_protocol::evidence::{
    AuditIngestionProof, ExternalAuditEvidenceBatch, ExternalEvidencePort,
    EXTERNAL_EVIDENCE_INGESTION_PROOF_SCHEMA, EXTERNAL_EVIDENCE_SCHEMA,
};
use cynic_protocol::EvidenceEnvelope;

pub struct LocalExternalEvidenceAdapter<S> {
    pub store: S,
}

impl<S> LocalExternalEvidenceAdapter<S>
where
    S: ContentAddressedEvidenceStorePort,
{
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

impl<S> ExternalEvidencePort for LocalExternalEvidenceAdapter<S>
where
    S: ContentAddressedEvidenceStorePort,
{
    fn ingest(&self, batch: &ExternalAuditEvidenceBatch) -> AuditIngestionProof {
        let envelope = EvidenceEnvelope {
            schema: EXTERNAL_EVIDENCE_SCHEMA.to_string(),
            digest: String::new(),
            payload_digest: String::new(),
            subject: batch.subject.clone(),
            producer: batch.producer.clone(),
            observed_at_unix_ms: batch.observed_at_unix_ms,
            provenance: batch.provenance.clone(),
            lineage: None,
            payload: batch.payload.clone(),
        };

        match self.store.persist(&envelope) {
            Ok(digest) => AuditIngestionProof {
                schema: EXTERNAL_EVIDENCE_INGESTION_PROOF_SCHEMA,
                project_id: batch.project_id.clone(),
                batches_ingested: 1,
                batches_failed: 0,
                digests: vec![digest],
                overall_success: true,
            },
            Err(e) => AuditIngestionProof {
                schema: EXTERNAL_EVIDENCE_INGESTION_PROOF_SCHEMA,
                project_id: batch.project_id.clone(),
                batches_ingested: 0,
                batches_failed: 1,
                digests: vec![],
                overall_success: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::evidence_store_port::ContentAddressedEvidenceStorePort;

    struct MockStore;
    impl ContentAddressedEvidenceStorePort for MockStore {
        fn persist(&self, _envelope: &EvidenceEnvelope) -> Result<String, String> {
            Ok("mock-digest-123".to_string())
        }
    }

    #[test]
    fn ingest_persists() {
        let adapter = LocalExternalEvidenceAdapter::new(MockStore);
        let batch = ExternalAuditEvidenceBatch {
            project_id: "aegisscan".to_string(),
            producer: "aegisscan-findings-v2".to_string(),
            subject: "pentest-run-1".to_string(),
            observed_at_unix_ms: 1000,
            provenance: vec![],
            payload: serde_json::json!({"finding": "test"}),
        };
        let result = adapter.ingest(&batch);
        assert!(result.overall_success);
        assert_eq!(result.batches_ingested, 1);
        assert_eq!(result.digests, vec!["mock-digest-123".to_string()]);
    }
}
