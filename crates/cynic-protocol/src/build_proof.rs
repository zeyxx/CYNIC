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
    /// Test counts parsed from cargo test stdout on success. None when no
    /// test phase ran or when the output carried no countable line —
    /// unknown, never zero-filled (a zero would fake a vacuous pass).
    #[serde(default)]
    pub tests_passed: Option<u64>,
    /// See `tests_passed`.
    #[serde(default)]
    pub tests_failed: Option<u64>,
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
            tests_passed: None,
            tests_failed: None,
        }
    }
}

/// Parse cargo test counts from runner stdout.
///
/// Sums every `test result: ok. N passed; M failed; ...` line (one per test
/// binary). Returns (None, None) when no countable line exists — the caller
/// reports unknown, never a zero that would bless a vacuous filter match.
pub fn parse_cargo_test_counts(stdout: &str) -> (Option<u64>, Option<u64>) {
    let mut passed: u64 = 0;
    let mut failed: u64 = 0;
    let mut lines = 0;
    for line in stdout.lines() {
        let Some(rest) = line.split_once("test result:") else {
            continue;
        };
        let Some(p) = count_before_word(rest, "passed") else {
            continue;
        };
        let Some(f) = count_before_word(rest, "failed") else {
            continue;
        };
        passed = passed.saturating_add(p);
        failed = failed.saturating_add(f);
        lines += 1;
    }
    if lines == 0 {
        (None, None)
    } else {
        (Some(passed), Some(failed))
    }
}

/// Largest u64 immediately preceding `word` in `text` (e.g. `7` in
/// `"ok. 7 passed"`). None when absent.
fn count_before_word(text: &str, word: &str) -> Option<u64> {
    let idx = text.find(word)?;
    text[..idx]
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .last()?
        .parse()
        .ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_sum_across_binaries() {
        let stdout = "running 7 tests\ntest result: ok. 7 passed; 0 failed; 0 ignored\n\
                      running 3 tests\ntest result: ok. 3 passed; 0 failed; 0 ignored\n";
        assert_eq!(parse_cargo_test_counts(stdout), (Some(10), Some(0)));
    }

    #[test]
    fn no_countable_line_is_unknown_not_zero() {
        assert_eq!(
            parse_cargo_test_counts(" Compiling foo\n Finished\n"),
            (None, None)
        );
        assert_eq!(
            parse_cargo_test_counts("test result: ok. 0 passed; 0 failed; 10 filtered out\n"),
            (Some(0), Some(0))
        );
    }

    #[test]
    fn failures_are_counted() {
        let stdout = "test result: FAILED. 2 passed; 1 failed; 0 ignored\n";
        assert_eq!(parse_cargo_test_counts(stdout), (Some(2), Some(1)));
    }
}
