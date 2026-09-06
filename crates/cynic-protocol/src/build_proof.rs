//! Build proof capability — native CYNIC surface for reproducible build verification.
//!
//! Replaces ad-hoc SSH+shell proof sequences with a typed, portable port:
//!   1. observe (read-only): resolve runner, ensure per-SHA checkout, dry-run plan
//!   2. verify (bounded action): `cargo check` and/or `cargo test` on the exact SHA
//!   3. read-back proof: versioned JSON receipt (`cynic.build_proof_report.v1`)
//!
//! CLI, MCP, and agent workloads are projections over this surface. None of them
//! owns checkout, compilation, or proof policy.

use serde::{Deserialize, Serialize};

pub const BUILD_PROOF_REQUEST_SCHEMA: &str = "cynic.build_proof_request.v1";
pub const BUILD_PROOF_REPORT_SCHEMA: &str = "cynic.build_proof_report.v1";
pub const BUILD_PROOF_CARD_SCHEMA_V1: &str = "cynic.build_proof_card.v1";

/// Compact public-proof card (futarchic audience projection).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildProofCard {
    pub schema: String,
    pub card_id: String,
    pub epoch: u64,
    pub timestamp: String,
    pub commit_hash: String,
    pub crates_affected: Vec<String>,
    pub lines_inserted: usize,
    pub lines_deleted: usize,
    pub audit_v2_status: String,
    pub q_score_bps: Option<u32>,
    pub proof_digest: String,
    pub public_summary: String,
}

/// What the caller asks the runner to verify.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildProofPhase {
    /// Type-check only: `cargo check -p <pkg> --lib`.
    Check,
    /// Check + focused test: `cargo check` then `cargo test --lib <filter>`.
    Test { filter: Option<String> },
}

/// One bounded proof request. No shell composition, no free argv.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildProofRequest {
    pub schema: String,
    /// Exact 40-character source commit to verify.
    pub source_commit: String,
    /// Package scope. None = workspace.
    pub package: Option<String>,
    pub phase: BuildProofPhase,
    /// Network mode for the verification.
    pub network_mode: BuildProofNetworkMode,
}

impl BuildProofRequest {
    pub fn new(
        source_commit: impl Into<String>,
        package: Option<String>,
        phase: BuildProofPhase,
    ) -> Self {
        Self {
            schema: BUILD_PROOF_REQUEST_SCHEMA.to_string(),
            source_commit: source_commit.into(),
            package,
            phase,
            network_mode: BuildProofNetworkMode::Offline,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildProofNetworkMode {
    Offline,
    Online,
}

/// Outcome of a bounded verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BuildProofStatus {
    Converged,
    /// Compilation or test failed; `detail` carries the tail.
    Failed {
        detail: String,
    },
    /// Runner unreachable or checkout could not be pinned.
    Unreachable {
        detail: String,
    },
}

/// Versioned proof receipt. The single source of truth that a commit built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildProofReport {
    pub schema: String,
    pub source_commit: String,
    pub package: Option<String>,
    pub phase: String,
    pub status: BuildProofStatus,
    /// Exact argv the runner executed (observability, not authority).
    pub argv: Vec<String>,
    /// Runner node identity.
    pub node_id: String,
    /// Wall-time seconds for the bounded action.
    pub elapsed_secs: u64,
}

impl BuildProofReport {
    pub fn converged(source_commit: impl Into<String>) -> Self {
        Self {
            schema: BUILD_PROOF_REPORT_SCHEMA.to_string(),
            source_commit: source_commit.into(),
            package: None,
            phase: "check".to_string(),
            status: BuildProofStatus::Converged,
            argv: Vec::new(),
            node_id: String::new(),
            elapsed_secs: 0,
        }
    }
}

/// Pure structural validation of a proof request.
pub fn validate_build_proof_request(request: &BuildProofRequest) -> Vec<String> {
    let mut blockers = Vec::new();
    if request.schema != BUILD_PROOF_REQUEST_SCHEMA {
        blockers.push("build_proof_request_schema_mismatch".to_string());
    }
    if request.source_commit.len() != 40
        || !request.source_commit.chars().all(|c| c.is_ascii_hexdigit())
    {
        blockers.push("build_proof_request_source_commit_invalid".to_string());
    }
    blockers
}
