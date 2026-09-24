//! Pure source-admission logic — evaluates gate proofs and a sync decision
//! to produce a [`SourceAdmissionReport`] verdict. Zero I/O.

use super::types::{SourceAdmissionReport, SyncDecision};

/// Pure admission rule — given gate proofs and a sync decision, produce
/// an admission verdict.
///
/// Blockers include:
/// - any failed mandatory gate (`cargo_check`, `secret_lint`, `git_clean`)
/// - `decision.needs_review == true` (large gap, dirty cortex)
/// - any `decision.unknowns` (unreachable remotes, missing identity)
pub fn admit_source_sync(
    decision: &SyncDecision,
    gate_passed: &[(&str, bool)],
) -> SourceAdmissionReport {
    let mut gates_evaluated = Vec::new();
    let mut gates_passed = Vec::new();
    let mut gates_failed = Vec::new();
    let mut blockers = Vec::new();

    for (name, passed) in gate_passed {
        gates_evaluated.push(name.to_string());
        if *passed {
            gates_passed.push(name.to_string());
        } else {
            gates_failed.push(name.to_string());
            blockers.push(format!("gate_failed:{name}"));
        }
    }

    if decision.needs_review {
        blockers.push("decision_needs_review".to_string());
    }

    for unknown in &decision.unknowns {
        blockers.push(format!("unknown:{unknown}"));
    }

    if blockers.is_empty() {
        SourceAdmissionReport::admitted(decision, gates_evaluated, gates_passed)
    } else {
        SourceAdmissionReport::blocked(decision, gates_passed, gates_failed, blockers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::types::{SourceLayer, SourceState, SyncObservation};

    fn decision(needs_review: bool, unknowns: Vec<String>) -> SyncDecision {
        SyncDecision {
            schema: "cynic.host.source_sync_decision.v1",
            observation: SyncObservation {
                schema: "test",
                cortex: SourceState {
                    layer: SourceLayer::Cortex,
                    remote_name: None,
                    remote_url: None,
                    head_sha: Some("abc".into()),
                    branch: Some("main".into()),
                    commits_ahead: Some(0),
                    commits_behind: Some(0),
                    dirty_files: vec![],
                    reachable: true,
                    blockers: vec![],
                },
                forgejo: SourceState {
                    layer: SourceLayer::Forgejo,
                    remote_name: Some("origin".into()),
                    remote_url: Some("ssh://forgejo@...".into()),
                    head_sha: Some("abc".into()),
                    branch: Some("main".into()),
                    commits_ahead: Some(0),
                    commits_behind: Some(0),
                    dirty_files: vec![],
                    reachable: true,
                    blockers: vec![],
                },
                github: SourceState {
                    layer: SourceLayer::GitHub,
                    remote_name: Some("github".into()),
                    remote_url: Some("https://github.com/...".into()),
                    head_sha: Some("abc".into()),
                    branch: Some("main".into()),
                    commits_ahead: None,
                    commits_behind: None,
                    dirty_files: vec![],
                    reachable: true,
                    blockers: vec![],
                },
                timestamp_unix_s: 0,
            },
            actions: vec![],
            needs_review,
            unknowns,
        }
    }

    #[test]
    fn all_gates_pass_and_clean_decision_admits() {
        let d = decision(false, vec![]);
        let gates = [
            ("cargo_check", true),
            ("secret_lint", true),
            ("git_clean", true),
        ];
        let report = admit_source_sync(&d, &gates);
        assert!(report.admitted);
        assert!(report.blockers.is_empty());
        assert_eq!(report.gates_passed.len(), 3);
    }

    #[test]
    fn failed_gate_blocks_admission() {
        let d = decision(false, vec![]);
        let gates = [
            ("cargo_check", false),
            ("secret_lint", true),
            ("git_clean", true),
        ];
        let report = admit_source_sync(&d, &gates);
        assert!(!report.admitted);
        assert_eq!(report.blockers.len(), 1);
        assert!(report.blockers[0].starts_with("gate_failed:"));
    }

    #[test]
    fn needs_review_blocks_admission() {
        let d = decision(true, vec![]);
        let gates = [
            ("cargo_check", true),
            ("secret_lint", true),
            ("git_clean", true),
        ];
        let report = admit_source_sync(&d, &gates);
        assert!(!report.admitted);
        assert!(report.blockers.iter().any(|b| b == "decision_needs_review"));
    }

    #[test]
    fn unknowns_block_admission() {
        let d = decision(false, vec!["forgejo remote is not reachable".into()]);
        let gates = [
            ("cargo_check", true),
            ("secret_lint", true),
            ("git_clean", true),
        ];
        let report = admit_source_sync(&d, &gates);
        assert!(!report.admitted);
        assert!(report.blockers.iter().any(|b| b.starts_with("unknown:")));
    }
}
