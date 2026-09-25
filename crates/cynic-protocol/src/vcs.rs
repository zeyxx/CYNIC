//! VCS and worktree protocol types for CYNIC.
//!
//! Layer 0 pure contracts: zero dependencies on host I/O or git processes.

use serde::{Deserialize, Serialize};
use crate::SecurityEvidenceState;

pub const SOURCE_MATERIALIZATION_FORMAT: &str = "git_bundle_v1";

/// Format canonical lease target identifier for an isolated source worktree.
pub fn source_worktree_lease_target(repository_id: &str, worktree_id: &str) -> String {
    format!("source-worktree:{repository_id}:{worktree_id}")
}

/// Pure snapshot of observed source state for admission and build ingestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSnapshot {
    pub branch: Option<String>,
    pub head_commit: Option<String>,
    pub upstream: Option<String>,
    pub staged_paths: Vec<String>,
    pub unstaged_paths: Vec<String>,
    pub untracked_paths: Vec<String>,
    pub unmerged_paths: Vec<String>,
    pub validation: SecurityEvidenceState,
    pub sovereign_transport_identity: SecurityEvidenceState,
    pub sovereign_authentication: SecurityEvidenceState,
    pub remote_commit_matches: SecurityEvidenceState,
    pub build_input_digest: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lease_target_format() {
        assert_eq!(
            source_worktree_lease_target("cynic", "agent-hermes"),
            "source-worktree:cynic:agent-hermes"
        );
    }
}
