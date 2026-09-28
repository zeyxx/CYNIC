//! Host adapter collecting gate proofs and producing a [`SourceAdmissionReport`]
//! for a source sync decision.

use std::path::Path;

use cynic_vcs::sync::{SourceAdmissionReport, SyncDecision};

use cynic_release::adapters::release_gate_adapter::collect_host_gate_proofs;

/// Collect gate proofs from the host and admit the sync decision.
///
/// Mandatory gates evaluated:
/// - `cargo_check` — workspace compiles
/// - `secret_lint` — no secrets in source
/// - `git_clean` — worktree clean (no dirty files)
pub fn admit_source_sync(root: &Path, decision: &SyncDecision) -> SourceAdmissionReport {
    let gate_proofs = collect_host_gate_proofs(root);

    let gate_passed: Vec<(&str, bool)> = gate_proofs
        .iter()
        .map(|record| (record.name.as_str(), record.passed))
        .collect();

    let gate_passed_slices: Vec<(&str, bool)> = gate_passed
        .iter()
        .map(|(name, passed)| (*name, *passed))
        .collect();

    cynic_vcs::sync::admit_source_sync(decision, &gate_passed_slices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_vcs::sync::types::{SourceLayer, SourceState, SyncObservation};

    fn decision_clean() -> SyncDecision {
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
            needs_review: false,
            unknowns: vec![],
        }
    }

    #[test]
    fn admission_reports_schema() {
        let d = decision_clean();
        // This will fail if the workspace isn't clean, but the schema should be set
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let report = admit_source_sync(&root, &d);
        assert_eq!(report.schema, "cynic.source_admission.v1");
    }
}
