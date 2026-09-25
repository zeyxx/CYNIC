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

pub use crate::source_control::SourceSnapshot;

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
