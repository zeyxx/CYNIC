//! Git branch topology observation — pure types, no I/O.
//!
//! Covers what `cynic source status` does not: the N-branch topology of the
//! local repository (ahead/behind per branch, worktree linkage, staleness).
//! All fields are observed read-back, never desired state.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const GIT_TOPOLOGY_SCHEMA: &str = "cynic.git_topology.v1";
pub const GIT_TOPOLOGY_MAX_BRANCHES: usize = 256;
pub const GIT_TOPOLOGY_MAX_STRING_BYTES: usize = 4096;
const MAX_OBSERVATION_UNKNOWNS: usize = 8;
const MAX_ENTRY_UNKNOWNS: usize = 6;

// ---------------------------------------------------------------------------
// State & unknowns
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitBranchState {
    /// < 50 behind, recent activity
    Active,
    /// Work done, few behind — ready for PR
    Stable,
    /// > 50 behind main
    Divergent,
    /// > 100 behind or recyclable in < 1d
    Dead,
    /// Local branch with no remote tracking (orphan)
    Orphan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitTopologyUnknown {
    BranchName,
    Ahead,
    Behind,
    LastCommit,
    Worktree,
    Dirty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitTopologyObservationUnknown {
    GitCommandFailed,
    EmptyOutput,
    OutputLimitExceeded,
    BranchLimitExceeded,
}

// ---------------------------------------------------------------------------
// Entries
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitBranchEntry {
    /// Fully qualified ref name, e.g. "agent/hermes/git-supervision"
    pub branch: String,
    /// Commits ahead of origin/main (0 if on main or orphan)
    #[serde(default)]
    pub ahead: Option<u32>,
    /// Commits behind origin/main
    #[serde(default)]
    pub behind: Option<u32>,
    /// Last commit unix seconds (0 = unknown)
    #[serde(default)]
    pub last_commit_unix_s: Option<u64>,
    /// Derived state from ahead/behind/last_commit
    pub state: GitBranchState,
    /// Has a linked worktree on this host
    #[serde(default)]
    pub has_worktree: Option<bool>,
    /// Dirty files count in that worktree (if observed)
    #[serde(default)]
    pub dirty_count: Option<u32>,
    /// Remote tracking exists
    #[serde(default)]
    pub has_remote: Option<bool>,
    #[serde(default)]
    pub unknowns: Vec<GitTopologyUnknown>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitTopologyObservation {
    pub schema: String,
    pub observed_at_unix_s: u64,
    pub node_id: String,
    /// Branch that is currently checked out (if any)
    #[serde(default)]
    pub current_branch: Option<String>,
    #[serde(default)]
    pub entries: Vec<GitBranchEntry>,
    #[serde(default)]
    pub observation_unknowns: Vec<GitTopologyObservationUnknown>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

pub fn validate_git_topology_observation(obs: &GitTopologyObservation) -> Vec<String> {
    let mut v = Vec::new();
    if obs.schema != GIT_TOPOLOGY_SCHEMA {
        v.push("git_topology_schema_mismatch".to_string());
    }
    if obs.observed_at_unix_s == 0 {
        v.push("git_topology_observed_at_missing".to_string());
    }
    if obs.node_id.trim().is_empty() || obs.node_id.len() > GIT_TOPOLOGY_MAX_STRING_BYTES {
        v.push("git_topology_node_id_invalid".to_string());
    }
    if obs.entries.len() > GIT_TOPOLOGY_MAX_BRANCHES {
        v.push("git_topology_branch_limit_exceeded".to_string());
    }
    if obs.entries.is_empty() && obs.observation_unknowns.is_empty() {
        v.push("git_topology_coverage_unknown_unannotated".to_string());
    }
    if obs.observation_unknowns.len() > MAX_OBSERVATION_UNKNOWNS {
        v.push("git_topology_observation_unknown_limit_exceeded".to_string());
    }
    let mut seen = HashSet::new();
    for e in &obs.entries {
        if !seen.insert(&e.branch) {
            v.push("git_topology_branch_duplicate".to_string());
        }
        if e.branch.trim().is_empty() || e.branch.len() > GIT_TOPOLOGY_MAX_STRING_BYTES {
            v.push("git_topology_branch_name_invalid".to_string());
        }
        if e.branch.contains(' ') || e.branch.contains('\0') {
            v.push("git_topology_branch_name_invalid".to_string());
        }
        if e.unknowns.len() > MAX_ENTRY_UNKNOWNS {
            v.push("git_topology_entry_unknown_limit_exceeded".to_string());
        }
        let uniq: HashSet<_> = e.unknowns.iter().copied().collect();
        if uniq.len() != e.unknowns.len() {
            v.push("git_topology_entry_unknown_duplicate".to_string());
        }
        // Coherence: if field is None, its unknown must be present, and vice versa
        let checks: &[(bool, GitTopologyUnknown)] = &[
            (e.ahead.is_some(), GitTopologyUnknown::Ahead),
            (e.behind.is_some(), GitTopologyUnknown::Behind),
            (e.last_commit_unix_s.is_some(), GitTopologyUnknown::LastCommit),
            (e.has_worktree.is_some(), GitTopologyUnknown::Worktree),
            (e.dirty_count.is_some(), GitTopologyUnknown::Dirty),
        ];
        for (is_some, kind) in checks {
            let marked = e.unknowns.contains(kind);
            if *is_some && marked {
                v.push("git_topology_entry_unknown_contradictory".to_string());
            } else if !*is_some && !marked {
                v.push("git_topology_entry_unknown_unannotated".to_string());
            }
        }
    }
    let obs_unknown_uniq: HashSet<_> = obs.observation_unknowns.iter().copied().collect();
    if obs_unknown_uniq.len() != obs.observation_unknowns.len() {
        v.push("git_topology_observation_unknown_duplicate".to_string());
    }
    v.sort();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_entry(branch: &str) -> GitBranchEntry {
        GitBranchEntry {
            branch: branch.to_string(),
            ahead: Some(2),
            behind: Some(0),
            last_commit_unix_s: Some(1_700_000_000),
            state: GitBranchState::Active,
            has_worktree: Some(false),
            dirty_count: Some(0),
            has_remote: Some(true),
            unknowns: vec![],
        }
    }

    #[test]
    fn roundtrip() {
        let obs = GitTopologyObservation {
            schema: GIT_TOPOLOGY_SCHEMA.to_string(),
            observed_at_unix_s: 1_700_000_001,
            node_id: "cortex".to_string(),
            current_branch: Some("main".to_string()),
            entries: vec![valid_entry("agent/hermes/x")],
            observation_unknowns: vec![],
        };
        assert!(validate_git_topology_observation(&obs).is_empty());
        let json = serde_json::to_string(&obs).unwrap();
        let back: GitTopologyObservation = serde_json::from_str(&json).unwrap();
        assert_eq!(obs, back);
    }

    #[test]
    fn unknown_must_match_missing_field() {
        let mut e = valid_entry("agent/hermes/x");
        e.ahead = None; // missing but not annotated
        let obs = GitTopologyObservation {
            schema: GIT_TOPOLOGY_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            node_id: "n".to_string(),
            current_branch: None,
            entries: vec![e],
            observation_unknowns: vec![],
        };
        let v = validate_git_topology_observation(&obs);
        assert!(v.contains(&"git_topology_entry_unknown_unannotated".to_string()));
    }
}
