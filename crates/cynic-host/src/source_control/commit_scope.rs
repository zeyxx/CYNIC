use std::collections::BTreeSet;

use cynic_protocol::SourceSnapshot;

pub(super) fn preflight_commit_scope(
    snapshot: &SourceSnapshot,
    proposed_paths: &[String],
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !snapshot.staged_paths.is_empty() {
        blockers.push("source_index_not_clean_before_commit".to_string());
    }

    if proposed_paths.is_empty() {
        blockers.push("source_change_has_no_observed_paths".to_string());
    }
    // Unstaged or untracked paths outside the proposal remain visible in the
    // before/after snapshot. They are not part of this mutation: `commit`
    // stages only proposal paths and the post-commit tree read-back below
    // requires that exact set. Blocking on unrelated dirty paths would make
    // a scoped commit impossible in a shared agent worktree.
    blockers
}

pub(super) fn commit_read_back_blockers(
    before_commit: Option<&str>,
    after_commit: Option<&str>,
    committed_paths: &[String],
    proposed_paths: &[String],
) -> Vec<String> {
    let mut blockers = Vec::new();
    if before_commit.is_none() || after_commit.is_none() || before_commit == after_commit {
        blockers.push("source_commit_read_back_missing_or_unchanged".to_string());
    }
    let committed = committed_paths.iter().collect::<BTreeSet<_>>();
    let proposed = proposed_paths.iter().collect::<BTreeSet<_>>();
    if committed != proposed {
        blockers.push("source_commit_scope_read_back_mismatch".to_string());
    }
    blockers
}

#[cfg(test)]
mod tests {
    use cynic_protocol::{SecurityEvidenceState, SourceSnapshot};

    use super::{commit_read_back_blockers, preflight_commit_scope};

    fn snapshot(staged: &[&str], unstaged: &[&str], untracked: &[&str]) -> SourceSnapshot {
        SourceSnapshot {
            branch: Some("agent/test".to_string()),
            head_commit: Some("a".repeat(40)),
            upstream: None,
            staged_paths: staged.iter().map(|path| (*path).to_string()).collect(),
            unstaged_paths: unstaged.iter().map(|path| (*path).to_string()).collect(),
            untracked_paths: untracked.iter().map(|path| (*path).to_string()).collect(),
            unmerged_paths: Vec::new(),
            validation: SecurityEvidenceState::Proven,
            sovereign_transport_identity: SecurityEvidenceState::Unknown,
            sovereign_authentication: SecurityEvidenceState::Unknown,
            remote_commit_matches: SecurityEvidenceState::Unknown,
            build_input_digest: None,
            evidence_refs: Vec::new(),
        }
    }

    #[test]
    fn preflight_rejects_a_contaminated_index_but_allows_unrelated_dirty_paths() {
        let blockers = preflight_commit_scope(
            &snapshot(&["other.rs"], &["claimed.rs", "other.rs"], &[]),
            &["claimed.rs".to_string()],
        );
        assert!(blockers.contains(&"source_index_not_clean_before_commit".to_string()));
        assert!(!blockers
            .iter()
            .any(|blocker| blocker.starts_with("source_change_path_outside_proposal:")));
    }

    #[test]
    fn read_back_requires_a_new_commit_with_the_exact_proposed_paths() {
        let blockers = commit_read_back_blockers(
            Some("a"),
            Some("b"),
            &["claimed.rs".to_string(), "other.rs".to_string()],
            &["claimed.rs".to_string()],
        );
        assert!(blockers.contains(&"source_commit_scope_read_back_mismatch".to_string()));
    }
}
