//! Build pipeline vocabulary: pure types and value objects.
//! No I/O, no process spawn, no clock reads. Decisions live in runner.rs,
//! effects behind the ports in ports.rs.

use cynic_protocol::EvidenceEnvelope;
use serde::{Deserialize, Serialize};

use crate::build::{BuildInputAdmission, BuildRunnerAdmission, BuildRunnerEnvelope};

pub use cynic_protocol::build::conformance::{BuildCommandProof, BuildConformanceEvidence, BuildProof};

pub const BUILD_PROOF_SCHEMA: &str = cynic_protocol::build::BUILD_PROOF_SCHEMA;
pub const CANONICAL_GATE_READ_BACK_SCHEMA: &str = cynic_protocol::build::CANONICAL_GATE_READ_BACK_SCHEMA;

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
/// Outcome of one artifact installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInstall {
    pub installed_digest: String,
    pub backup_path: Option<String>,
}
