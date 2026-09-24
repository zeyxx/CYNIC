//! Pure data models and ports for Forgejo Pull Request governance.
//! Zero-I/O: models states, transitions, review evaluations, and admission criteria.

use serde::{Deserialize, Serialize};

pub const FORGEJO_PR_SCHEMA: &str = "cynic.forgejo.pull_request.v1";
pub const FORGEJO_PR_ADMISSION_SCHEMA: &str = "cynic.forgejo.pr_admission.v1";

/// Status of a Pull Request on Forgejo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgejoPrState {
    Open,
    Closed,
    Merged,
}

/// Representation of an observed Forgejo Pull Request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoPullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub state: ForgejoPrState,
    pub head_ref: String,
    pub head_sha: String,
    pub base_ref: String,
    pub base_sha: String,
    pub mergeable: bool,
    pub has_conflicts: bool,
    pub html_url: String,
}

/// Admission decision for a PR merge attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoPrAdmissionReport {
    pub schema: &'static str,
    pub pr_number: u64,
    pub head_ref: String,
    pub admitted: bool,
    pub blockers: Vec<String>,
    pub required_checks: Vec<String>,
}

/// Pure Zero-I/O evaluator for PR merge admission.
pub fn evaluate_pr_admission(
    pr: &ForgejoPullRequest,
    builder_check_passed: bool,
    secret_lint_passed: bool,
) -> ForgejoPrAdmissionReport {
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

    if pr.state != ForgejoPrState::Open {
        blockers.push(format!("pr_not_open:state_{:?}", pr.state));
    }

    if pr.has_conflicts || !pr.mergeable {
        blockers.push("merge_conflict_detected".to_string());
    }

    let admitted = blockers.is_empty();

    ForgejoPrAdmissionReport {
        schema: FORGEJO_PR_ADMISSION_SCHEMA,
        pr_number: pr.number,
        head_ref: pr.head_ref.clone(),
        admitted,
        blockers,
        required_checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admit_clean_pr() {
        let pr = ForgejoPullRequest {
            id: 42,
            number: 10,
            title: "feat: safe capability".into(),
            state: ForgejoPrState::Open,
            head_ref: "agent/hermes/safe".into(),
            head_sha: "abcd123".into(),
            base_ref: "main".into(),
            base_sha: "1234abc".into(),
            mergeable: true,
            has_conflicts: false,
            html_url: "http://10.10.10.101:3000/jeanterre13/CYNIC/pulls/10".into(),
        };

        let verdict = evaluate_pr_admission(&pr, true, true);
        assert!(verdict.admitted);
        assert!(verdict.blockers.is_empty());
    }

    #[test]
    fn test_refuse_failing_builder_pr() {
        let pr = ForgejoPullRequest {
            id: 42,
            number: 10,
            title: "feat: broken build".into(),
            state: ForgejoPrState::Open,
            head_ref: "agent/hermes/broken".into(),
            head_sha: "abcd123".into(),
            base_ref: "main".into(),
            base_sha: "1234abc".into(),
            mergeable: true,
            has_conflicts: false,
            html_url: "http://10.10.10.101:3000/jeanterre13/CYNIC/pulls/10".into(),
        };

        let verdict = evaluate_pr_admission(&pr, false, true);
        assert!(!verdict.admitted);
        assert_eq!(verdict.blockers, vec!["builder_gate_unproven_or_failed"]);
    }
}
