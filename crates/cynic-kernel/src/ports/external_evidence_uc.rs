//! Pure orchestration usecase for external evidence ingestion.
//!
//! WHY: composes the `ExternalEvidencePort` trait with pure admission
//! logic to orchestrate evidence batch ingestion. This module is the
//! single call-site that ties port + admission + proof building together.
//! HOW: pure functions only, no I/O. All side effects (storage) live
//! behind the port trait.
//!
//! FCIS: this module is pure orchestration. No std::fs, no network,
//! no database. The port trait abstracts all I/O.

use super::external_evidence::{
    ExternalEvidenceBatch, ExternalEvidenceIngestionProof, ExternalEvidenceIngestionResult,
    ExternalEvidencePort, admit_external_evidence, build_ingestion_proof,
};

/// Orchestrate ingestion of a batch of external evidence through a port.
///
/// Pure delegation: calls `port.ingest_batch` and returns the proof.
/// No admission check here — callers run `admit_external_evidence` first
/// if they need admission gating.
///
/// # Arguments
/// * `port` — the evidence port (storage adapter)
/// * `project_id` — stable project slug
/// * `batches` — evidence batches to ingest
///
/// # Returns
/// The ingestion proof from the port.
pub fn ingest_batch_proof(
    port: &dyn ExternalEvidencePort,
    project_id: &str,
    batches: &[ExternalEvidenceBatch],
) -> ExternalEvidenceIngestionProof {
    port.ingest_batch(project_id, batches)
}

/// Orchestrate admission-gated batch ingestion.
///
/// Runs admission checks on each batch before delegating to the port.
/// Only admitted batches are forwarded to `port.ingest_batch`.
///
/// Pure: no I/O. Returns proof for admitted batches only.
pub fn ingest_batch_proof_admitted(
    port: &dyn ExternalEvidencePort,
    project_id: &str,
    batches: &[ExternalEvidenceBatch],
    sensitivity: &str,
) -> ExternalEvidenceIngestionProof {
    let admitted: Vec<ExternalEvidenceBatch> = batches
        .iter()
        .filter(|b| {
            admit_external_evidence(&b.project_id, &b.subject, &b.payload, sensitivity).admitted
        })
        .cloned()
        .collect();
    port.ingest_batch(project_id, &admitted)
}

/// Orchestrate batch ingestion with pre-built results (no port).
///
/// Useful when results are already available (e.g. cached, replayed)
/// and only the proof needs to be built.
///
/// Pure: no I/O. Delegates directly to `build_ingestion_proof`.
pub fn ingest_batch_proof_from_results(
    project_id: &str,
    results: Vec<ExternalEvidenceIngestionResult>,
) -> ExternalEvidenceIngestionProof {
    build_ingestion_proof(project_id, results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct MockPort;

    impl ExternalEvidencePort for MockPort {
        fn ingest(&self, batch: &ExternalEvidenceBatch) -> ExternalEvidenceIngestionResult {
            ExternalEvidenceIngestionResult {
                project_id: batch.project_id.clone(),
                subject: batch.subject.clone(),
                digest: format!("digest-{}", batch.subject),
                persisted: true,
                error: None,
            }
        }
    }

    struct FailingPort;

    impl ExternalEvidencePort for FailingPort {
        fn ingest(&self, batch: &ExternalEvidenceBatch) -> ExternalEvidenceIngestionResult {
            ExternalEvidenceIngestionResult {
                project_id: batch.project_id.clone(),
                subject: batch.subject.clone(),
                digest: String::new(),
                persisted: false,
                error: Some("storage unavailable".to_string()),
            }
        }
    }

    fn sample_batch(subject: &str) -> ExternalEvidenceBatch {
        ExternalEvidenceBatch {
            project_id: "aegisscan".to_string(),
            producer: "aegisscan-findings-v2".to_string(),
            subject: subject.to_string(),
            observed_at_unix_ms: 1000,
            provenance: vec!["provenance-1".to_string()],
            payload: json!({"finding": "test", "severity": "high"}),
        }
    }

    #[test]
    fn ingest_batch_proof_returns_port_proof() {
        let port = MockPort;
        let batches = vec![sample_batch("run-1"), sample_batch("run-2")];
        let proof = ingest_batch_proof(&port, "aegisscan", &batches);
        assert!(proof.overall_success);
        assert_eq!(proof.batches_ingested, 2);
        assert_eq!(proof.batches_failed, 0);
        assert_eq!(proof.digests.len(), 2);
        assert_eq!(proof.digests[0], "digest-run-1");
        assert_eq!(proof.digests[1], "digest-run-2");
    }

    #[test]
    fn ingest_batch_proof_empty_batches_not_success() {
        let port = MockPort;
        let batches: Vec<ExternalEvidenceBatch> = vec![];
        let proof = ingest_batch_proof(&port, "aegisscan", &batches);
        assert!(!proof.overall_success);
        assert_eq!(proof.batches_ingested, 0);
    }

    #[test]
    fn ingest_batch_proof_propagates_failures() {
        let port = FailingPort;
        let batches = vec![sample_batch("run-1")];
        let proof = ingest_batch_proof(&port, "aegisscan", &batches);
        assert!(!proof.overall_success);
        assert_eq!(proof.batches_ingested, 0);
        assert_eq!(proof.batches_failed, 1);
    }

    #[test]
    fn ingest_batch_proof_admitted_filters_empty_subject() {
        let port = MockPort;
        let mut bad_batch = sample_batch("run-1");
        bad_batch.subject = "   ".to_string();
        let batches = vec![bad_batch, sample_batch("run-2")];
        let proof = ingest_batch_proof_admitted(&port, "aegisscan", &batches, "public");
        assert!(proof.overall_success);
        assert_eq!(proof.batches_ingested, 1);
        assert_eq!(proof.digests.len(), 1);
        assert_eq!(proof.digests[0], "digest-run-2");
    }

    #[test]
    fn ingest_batch_proof_admitted_filters_empty_project() {
        let port = MockPort;
        let mut bad_batch = sample_batch("run-1");
        bad_batch.project_id = String::new();
        let batches = vec![bad_batch, sample_batch("run-2")];
        let proof = ingest_batch_proof_admitted(&port, "aegisscan", &batches, "public");
        assert!(proof.overall_success);
        assert_eq!(proof.batches_ingested, 1);
    }

    #[test]
    fn ingest_batch_proof_from_results_builds_proof() {
        let results = vec![
            ExternalEvidenceIngestionResult {
                project_id: "aegisscan".to_string(),
                subject: "run-1".to_string(),
                digest: "abc123".to_string(),
                persisted: true,
                error: None,
            },
            ExternalEvidenceIngestionResult {
                project_id: "aegisscan".to_string(),
                subject: "run-2".to_string(),
                digest: "def456".to_string(),
                persisted: false,
                error: Some("disk full".to_string()),
            },
        ];
        let proof = ingest_batch_proof_from_results("aegisscan", results);
        assert!(!proof.overall_success);
        assert_eq!(proof.batches_ingested, 1);
        assert_eq!(proof.batches_failed, 1);
        assert_eq!(proof.digests.len(), 2);
    }
}
