//! Build pipeline vocabulary: pure types and value objects.
//! No I/O, no process spawn, no clock reads. Decisions live in runner.rs,
//! effects behind the ports in ports.rs.

use cynic_protocol::EvidenceEnvelope;
use serde::{Deserialize, Serialize};

use crate::build::{BuildInputAdmission, BuildRunnerAdmission, BuildRunnerEnvelope};

pub const BUILD_PROOF_SCHEMA: &str = "cynic.host.build_proof.v1";
#[derive(Debug, Clone)]
pub struct BuildRequest {
    pub input: BuildInputAdmission,
    pub runner_envelope: BuildRunnerEnvelope,
    pub runner_admission: BuildRunnerAdmission,
    pub source_commit: String,
    pub argv: Vec<String>,
    pub artifact_path: String,
    /// Optional install target for the built artifact. None means the
    /// artifact is only digest-read-back, never installed.
    pub install_path: Option<String>,
    pub apply: bool,
}
pub const CANONICAL_GATE_READ_BACK_SCHEMA: &str = "cynic.build.canonical_gate_read_back.v1";
#[derive(Debug, Clone, Serialize)]
pub struct BuildCommandProof {
    pub argv: Vec<String>,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct BuildProof {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub source_commit: String,
    pub runner_node_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub consumed_actions: u64,
    pub artifact_path: String,
    pub artifact_digest: Option<String>,
    /// Canonical path the artifact was installed to, when requested.
    pub installed_path: Option<String>,
    /// Digest read back from the installed path. Must equal
    /// `artifact_digest` when present.
    pub installed_digest: Option<String>,
    /// True when a digest mismatch restored the `.prev` backup.
    pub rolled_back: bool,
    pub commands: Vec<BuildCommandProof>,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
}
/// Outcome of one artifact installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInstall {
    pub installed_digest: String,
    pub backup_path: Option<String>,
}
/// The narrow build fact consumed by runtime conformance.  It can originate
/// from an in-process build proof or from content-addressed scheduler evidence;
/// conformance never treats an arbitrary operator-supplied JSON file as proof.
#[derive(Debug, Clone, Serialize)]
pub struct BuildConformanceEvidence {
    pub schema: &'static str,
    pub status: &'static str,
    pub source_commit: String,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CanonicalGateReadBack {
    schema: String,
    source_commit: String,
    argv: Vec<String>,
    exit_code: Option<i32>,
}

impl BuildConformanceEvidence {
    pub fn from_build_proof(proof: &BuildProof) -> Self {
        Self {
            schema: "cynic.host.build_conformance_evidence.v1",
            status: match proof.status {
                "proved" => "proved",
                "failed" => "failed",
                _ => "unknown",
            },
            source_commit: proof.source_commit.clone(),
            evidence_refs: proof.evidence_refs.clone(),
            blockers: proof.blockers.clone(),
        }
    }

    pub fn from_content_addressed_envelope(
        requested_digest: &str,
        envelope: &EvidenceEnvelope,
    ) -> Result<Self, String> {
        if envelope.digest != requested_digest {
            return Err("build_evidence_digest_mismatch".to_string());
        }
        if !envelope.subject.starts_with("build-workload:")
            || !envelope.producer.starts_with("cynic-node/build-scheduler:")
        {
            return Err("build_evidence_provenance_rejected".to_string());
        }
        let read_back: CanonicalGateReadBack = serde_json::from_value(envelope.payload.clone())
            .map_err(|_| "build_evidence_payload_invalid".to_string())?;
        if read_back.schema != CANONICAL_GATE_READ_BACK_SCHEMA
            || read_back.source_commit.trim().is_empty()
            || read_back.argv.is_empty()
            || read_back
                .argv
                .iter()
                .any(|argument| argument.trim().is_empty())
        {
            return Err("build_evidence_contract_rejected".to_string());
        }
        let (status, blockers) = if read_back.exit_code == Some(0) {
            ("proved", Vec::new())
        } else {
            (
                "failed",
                vec![format!(
                    "canonical_gate_exit_code:{:?}",
                    read_back.exit_code
                )],
            )
        };
        Ok(Self {
            schema: "cynic.host.build_conformance_evidence.v1",
            status,
            source_commit: read_back.source_commit,
            evidence_refs: vec![requested_digest.to_string()],
            blockers,
        })
    }
}
