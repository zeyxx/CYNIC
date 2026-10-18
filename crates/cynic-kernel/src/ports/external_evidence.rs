//! Pure external evidence governance for non-CYNIC projects.
//!
//! WHY: external projects (aegisscan) produce findings/evidence that must be
//! ingested into CYNIC's observation store (K1 immutable evidence). This
//! module defines the `ExternalEvidencePort` trait and pure types that abstract
//! evidence ingestion without coupling to storage I/O.
//! HOW: trait + pure data types + pure admission validators. I/O (storage)
//! lives in adapters (cynic-host).
//! NEGATIVE BOUNDARY: no std::fs, no database I/O, no network. No dependency
//! on any external project. The kernel is the depositary of observations.

use serde::{Deserialize, Serialize};

use cynic_protocol::EvidenceEnvelope;

// ── Schemas ───────────────────────────────────────────────────────────────────

pub const EXTERNAL_EVIDENCE_SCHEMA: &str = "cynic.kernel.external_evidence.v1";
pub const EXTERNAL_EVIDENCE_INGESTION_PROOF_SCHEMA: &str =
    "cynic.kernel.external_evidence_ingestion_proof.v1";

// ── Core types ────────────────────────────────────────────────────────────────

/// Reference to an external evidence batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalEvidenceBatch {
    /// Stable project slug (e.g. "aegisscan").
    pub project_id: String,
    /// Producer identifier (e.g. "aegisscan-findings-v2").
    pub producer: String,
    /// Subject of the evidence (e.g. "pentest-run-123").
    pub subject: String,
    /// Observed timestamp (unix ms).
    pub observed_at_unix_ms: u128,
    /// Provenance chain (source refs).
    pub provenance: Vec<String>,
    /// Payload: project-specific findings (opaque to the kernel).
    pub payload: serde_json::Value,
}

/// Result of ingesting one evidence batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalEvidenceIngestionResult {
    pub project_id: String,
    pub subject: String,
    pub digest: String,
    pub persisted: bool,
    pub error: Option<String>,
}

/// Proof emitted after ingesting external evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalEvidenceIngestionProof {
    pub schema: &'static str,
    pub project_id: String,
    pub batches_ingested: usize,
    pub batches_failed: usize,
    pub digests: Vec<String>,
    pub overall_success: bool,
}

// ── Admission (pure) ──────────────────────────────────────────────────────────

/// Admission report for an external evidence ingestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalEvidenceAdmission {
    pub admitted: bool,
    pub blockers: Vec<String>,
}

/// Validate that a project is eligible for evidence ingestion.
///
/// Pure: no I/O. Checks project_id non-empty, payload non-empty.
pub fn admit_external_evidence(
    project_id: &str,
    subject: &str,
    payload: &serde_json::Value,
    sensitivity: &str,
) -> ExternalEvidenceAdmission {
    let mut blockers = Vec::new();

    if project_id.is_empty() {
        blockers.push("project_id is empty".to_string());
    }

    if subject.trim().is_empty() {
        blockers.push("subject is empty".to_string());
    }

    if payload.is_null() || (payload.is_object() && payload.as_object().unwrap().is_empty()) {
        blockers.push("payload is empty".to_string());
    }

    // sanitized projects: evidence ingestion allowed (read-only observation)
    // but no deployment or arming
    if sensitivity == "sanitized" {
        // allowed: evidence ingestion is observation, not mutation
    }

    ExternalEvidenceAdmission {
        admitted: blockers.is_empty(),
        blockers,
    }
}

// ── Pure ingestion builder ────────────────────────────────────────────────────

/// Build an ingestion proof from results. Pure, no I/O.
pub fn build_ingestion_proof(
    project_id: &str,
    results: Vec<ExternalEvidenceIngestionResult>,
) -> ExternalEvidenceIngestionProof {
    let batches_ingested = results.iter().filter(|r| r.persisted).count();
    let batches_failed = results.len() - batches_ingested;
    let digests: Vec<String> = results.iter().map(|r| r.digest.clone()).collect();
    let overall_success = batches_failed == 0 && !results.is_empty();

    ExternalEvidenceIngestionProof {
        schema: EXTERNAL_EVIDENCE_INGESTION_PROOF_SCHEMA,
        project_id: project_id.to_string(),
        batches_ingested,
        batches_failed,
        digests,
        overall_success,
    }
}

// ── Port trait ────────────────────────────────────────────────────────────────

/// Port for ingesting external evidence into CYNIC's observation store.
///
/// Abstracts evidence persistence. Concrete implementations (local evidence
/// store, content-addressed store) live in adapter crates (cynic-host).
/// No I/O in this trait definition.
pub trait ExternalEvidencePort: Send + Sync {
    /// Ingest one evidence batch and return the ingestion result.
    fn ingest(&self, batch: &ExternalEvidenceBatch) -> ExternalEvidenceIngestionResult;

    /// Ingest multiple batches and return the proof.
    fn ingest_batch(
        &self,
        project_id: &str,
        batches: &[ExternalEvidenceBatch],
    ) -> ExternalEvidenceIngestionProof {
        let results: Vec<ExternalEvidenceIngestionResult> =
            batches.iter().map(|b| self.ingest(b)).collect();
        build_ingestion_proof(project_id, results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admit_valid_batch_passes() {
        let payload = serde_json::json!({"finding": "test"});
        let r = admit_external_evidence("aegisscan", "pentest-run-1", &payload, "public");
        assert!(r.admitted);
    }

    #[test]
    fn admit_empty_project_id_blocked() {
        let payload = serde_json::json!({"finding": "test"});
        let r = admit_external_evidence("", "pentest-run-1", &payload, "public");
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("project_id is empty")));
    }

    #[test]
    fn admit_empty_subject_blocked() {
        let payload = serde_json::json!({"finding": "test"});
        let r = admit_external_evidence("aegisscan", "  ", &payload, "public");
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("subject is empty")));
    }

    #[test]
    fn admit_empty_payload_blocked() {
        let payload = serde_json::json!({});
        let r = admit_external_evidence("aegisscan", "pentest-run-1", &payload, "public");
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("payload is empty")));
    }

    #[test]
    fn admit_null_payload_blocked() {
        let payload = serde_json::Value::Null;
        let r = admit_external_evidence("aegisscan", "pentest-run-1", &payload, "public");
        assert!(!r.admitted);
    }

    #[test]
    fn ingestion_proof_all_pass() {
        let results = vec![
            ExternalEvidenceIngestionResult {
                project_id: "aegisscan".to_string(),
                subject: "run-1".to_string(),
                digest: "abc123".to_string(),
                persisted: true,
                error: None,
            },
        ];
        let proof = build_ingestion_proof("aegisscan", results);
        assert!(proof.overall_success);
        assert_eq!(proof.batches_ingested, 1);
        assert_eq!(proof.batches_failed, 0);
    }

    #[test]
    fn ingestion_proof_one_fail() {
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
                error: Some("storage full".to_string()),
            },
        ];
        let proof = build_ingestion_proof("aegisscan", results);
        assert!(!proof.overall_success);
        assert_eq!(proof.batches_ingested, 1);
        assert_eq!(proof.batches_failed, 1);
    }

    #[test]
    fn ingestion_proof_empty_not_success() {
        let proof = build_ingestion_proof("aegisscan", vec![]);
        assert!(!proof.overall_success);
        assert_eq!(proof.batches_ingested, 0);
    }

    #[test]
    fn port_trait_object_safe() {
        struct MockPort;
        impl ExternalEvidencePort for MockPort {
            fn ingest(&self, batch: &ExternalEvidenceBatch) -> ExternalEvidenceIngestionResult {
                ExternalEvidenceIngestionResult {
                    project_id: batch.project_id.clone(),
                    subject: batch.subject.clone(),
                    digest: "mock-digest".to_string(),
                    persisted: true,
                    error: None,
                }
            }
        }
        let port: Box<dyn ExternalEvidencePort> = Box::new(MockPort);
        let batch = ExternalEvidenceBatch {
            project_id: "aegisscan".to_string(),
            producer: "aegisscan-findings-v2".to_string(),
            subject: "pentest-run-1".to_string(),
            observed_at_unix_ms: 1000,
            provenance: vec![],
            payload: serde_json::json!({"finding": "test"}),
        };
        let result = port.ingest(&batch);
        assert!(result.persisted);
    }
}