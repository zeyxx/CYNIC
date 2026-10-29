use cynic_protocol::evidence::{
    admit_external_evidence, build_ingestion_proof, ExternalAuditEvidenceBatch, AuditIngestionProof,
    ExternalEvidencePort,
};

pub fn ingest_batch_proof(
    port: &dyn ExternalEvidencePort,
    project_id: &str,
    batches: &[ExternalAuditEvidenceBatch],
) -> AuditIngestionProof {
    port.ingest_batch(project_id, batches)
}

pub fn ingest_batch_proof_admitted(
    port: &dyn ExternalEvidencePort,
    project_id: &str,
    batches: &[ExternalAuditEvidenceBatch],
    sensitivity: &str,
) -> AuditIngestionProof {
    let admitted: Vec<ExternalAuditEvidenceBatch> = batches
        .iter()
        .filter(|b| {
            admit_external_evidence(&b.project_id, &b.subject, &b.payload, sensitivity).admitted
        })
        .cloned()
        .collect();
    port.ingest_batch(project_id, &admitted)
}

pub fn ingest_batch_proof_from_results(
    project_id: &str,
    results: Vec<AuditIngestionProof>,
) -> AuditIngestionProof {
    build_ingestion_proof(project_id, results)
}
