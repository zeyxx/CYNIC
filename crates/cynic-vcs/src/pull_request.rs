//! Pure data models and ports for sovereign Pull Request governance.
//!
//! This module is the domain core — Zero-I/O, host-agnostic.
//! Unaware of Forgejo, GitLab, or GitHub. Concrete adapters live in
//! this module (cynic-vcs::providers, …).
//!
//! Conforms to contracts/source-lifecycle.json (cynic.source_lifecycle.v1).

use serde::{Deserialize, Serialize};

// ── Schemas ───────────────────────────────────────────────────────────────────

pub const PR_SCHEMA: &str = "cynic.vcs.pull_request.v1";
pub const PR_ADMISSION_SCHEMA: &str = "cynic.vcs.pr_admission.v1";
pub const PR_CREATE_PROOF_SCHEMA: &str = "cynic.vcs.pr_create_proof.v1";
pub const PR_MERGE_PROOF_SCHEMA: &str = "cynic.vcs.pr_merge_proof.v1";
pub const PR_CLOSE_PROOF_SCHEMA: &str = "cynic.vcs.pr_close_proof.v1";

// ── Core types (provider-agnostic) ────────────────────────────────────────────

/// Lifecycle state of a Pull Request, normalised across VCS providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrState {
    Open,
    Closed,
    Merged,
}

/// Canonical PR representation returned by any `PrPort` implementor.
/// Fields map to the lowest common denominator across Forgejo, GitLab, GitHub.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub state: PrState,
    pub head_ref: String,
    pub head_sha: String,
    pub base_ref: String,
    pub base_sha: String,
    pub mergeable: bool,
    pub has_conflicts: bool,
    /// Canonical web URL (usable regardless of provider).
    pub html_url: String,
}

/// Proof emitted after a PR is created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrCreateProof {
    pub schema: &'static str,
    pub pr_number: u64,
    pub html_url: String,
    pub head_ref: String,
    pub base_ref: String,
}

/// Proof emitted after a PR is merged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrMergeProof {
    pub schema: &'static str,
    pub pr_number: u64,
    pub merged: bool,
    pub message: String,
}

/// Proof emitted after a PR is closed without merge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrCloseProof {
    pub schema: &'static str,
    pub pr_number: u64,
    pub closed: bool,
    pub branch_deleted: bool,
    pub message: String,
}

/// Admission verdict for a PR merge attempt. Pure, deterministic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrAdmissionReport {
    pub schema: &'static str,
    pub pr_number: u64,
    pub head_ref: String,
    pub admitted: bool,
    pub blockers: Vec<String>,
    pub required_checks: Vec<String>,
}

// ── Port ──────────────────────────────────────────────────────────────────────

/// Driven port for Pull Request actuation.
///
/// Implementations live in cynic-vcs::providers.
/// The domain only depends on this trait — never on a concrete adapter.
pub trait PrPort: Send + Sync {
    /// List PRs filtered by state ("open", "closed", "all").
    fn list(&self, state: &str) -> Result<Vec<PullRequest>, String>;

    /// Open a new PR and return a creation proof.
    fn open(
        &self,
        title: &str,
        head: &str,
        base: &str,
        body: &str,
    ) -> Result<PrCreateProof, String>;

    /// Merge an open PR by number and return a merge proof.
    /// `style` is provider-specific ("merge", "squash", "rebase").
    fn merge(&self, number: u64, style: &str) -> Result<PrMergeProof, String>;

    /// Close an open PR without merging and return a close proof.
    /// `delete_branch` removes the head branch after close when true.
    fn close(&self, number: u64, delete_branch: bool) -> Result<PrCloseProof, String>;
}

// ── Pure evaluator ────────────────────────────────────────────────────────────

/// Evaluate whether a PR is eligible for merge.
/// Fully pure — no I/O. The caller supplies gate results as plain booleans.
pub fn evaluate_pr_admission(
    pr: &PullRequest,
    builder_check_passed: bool,
    secret_lint_passed: bool,
) -> PrAdmissionReport {
    let mut blockers = Vec::new();
    let mut required_checks = Vec::new();

    required_checks.push("builder_check_passed".to_string());
    if !builder_check_passed {
        blockers.push("builder_gate_unproven_or_failed".to_string());
    }

    required_checks.push("secret_lint_passed".to_string());
    if !secret_lint_passed {
        blockers.push("secret_lint_violations_detected".to_string());
    }

    if pr.state != PrState::Open {
        blockers.push(format!("pr_not_open:state_{:?}", pr.state));
    }

    if pr.has_conflicts || !pr.mergeable {
        blockers.push("merge_conflict_detected".to_string());
    }

    PrAdmissionReport {
        schema: PR_ADMISSION_SCHEMA,
        pr_number: pr.number,
        head_ref: pr.head_ref.clone(),
        admitted: blockers.is_empty(),
        blockers,
        required_checks,
    }
}

/// Evaluate whether a PR is eligible for close without merge.
/// Fully pure — no I/O. Only an open PR may be closed; merged or already
/// closed PRs are refused with the same vocabulary as merge admission.
pub fn evaluate_pr_close(pr: &PullRequest) -> Result<(), String> {
    if pr.state != PrState::Open {
        return Err(format!("pr_not_open:state_{:?}", pr.state));
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn open_pr(number: u64) -> PullRequest {
        PullRequest {
            number,
            title: "feat: safe capability".into(),
            state: PrState::Open,
            head_ref: "agent/hermes/safe".into(),
            head_sha: "abcd123".into(),
            base_ref: "main".into(),
            base_sha: "1234abc".into(),
            mergeable: true,
            has_conflicts: false,
            html_url: format!("http://forgejo.service.cynic/jeanterre13/CYNIC/pulls/{number}"),
        }
    }

    #[test]
    fn admit_clean_pr() {
        let verdict = evaluate_pr_admission(&open_pr(10), true, true);
        assert!(verdict.admitted, "blockers: {:?}", verdict.blockers);
        assert!(verdict.blockers.is_empty());
    }

    #[test]
    fn refuse_failing_builder() {
        let verdict = evaluate_pr_admission(&open_pr(10), false, true);
        assert!(!verdict.admitted);
        assert_eq!(verdict.blockers, vec!["builder_gate_unproven_or_failed"]);
    }

    #[test]
    fn refuse_secret_lint_violation() {
        let verdict = evaluate_pr_admission(&open_pr(10), true, false);
        assert!(!verdict.admitted);
        assert_eq!(verdict.blockers, vec!["secret_lint_violations_detected"]);
    }

    #[test]
    fn refuse_all_gates_failed_reports_two_blockers() {
        let verdict = evaluate_pr_admission(&open_pr(10), false, false);
        assert!(!verdict.admitted);
        assert_eq!(verdict.blockers.len(), 2, "expected 2 blockers, got: {:?}", verdict.blockers);
    }

    #[test]
    fn refuse_conflict() {
        let mut pr = open_pr(10);
        pr.has_conflicts = true;
        pr.mergeable = false;
        let verdict = evaluate_pr_admission(&pr, true, true);
        assert!(!verdict.admitted);
        assert!(verdict.blockers.contains(&"merge_conflict_detected".to_string()));
    }

    #[test]
    fn refuse_non_open_pr() {
        let mut pr = open_pr(10);
        pr.state = PrState::Merged;
        let verdict = evaluate_pr_admission(&pr, true, true);
        assert!(!verdict.admitted);
        assert!(verdict.blockers.iter().any(|b| b.starts_with("pr_not_open:")));
    }

    #[test]
    fn admit_close_of_open_pr() {
        assert!(evaluate_pr_close(&open_pr(11)).is_ok());
    }

    #[test]
    fn refuse_close_of_merged_pr() {
        let mut pr = open_pr(11);
        pr.state = PrState::Merged;
        let err = evaluate_pr_close(&pr).expect_err("merged PR must refuse close");
        assert!(err.starts_with("pr_not_open:"), "unexpected: {err}");
    }

    #[test]
    fn refuse_close_of_closed_pr() {
        let mut pr = open_pr(11);
        pr.state = PrState::Closed;
        let err = evaluate_pr_close(&pr).expect_err("closed PR must refuse close");
        assert!(err.starts_with("pr_not_open:"), "unexpected: {err}");
    }
}
