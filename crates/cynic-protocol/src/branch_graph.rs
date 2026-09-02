use serde::{Deserialize, Serialize};

pub const BRANCH_GRAPH_SCHEMA: &str = "cynic.branch_graph.v1";

/// Stale threshold is policy, not stored bool — 48h for checkouts, 30d for branches.
pub const BRANCH_STALE_AFTER_SECS: u64 = 30 * 24 * 3600;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchGraph {
    pub schema: String,
    pub root: String,
    pub root_commit: String,
    pub nodes: Vec<BranchNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchNode {
    pub name: String,
    pub commit: String,
    pub parent: Option<String>,
    pub has_upstream: bool,
    pub last_commit_unix_s: Option<u64>,
}

pub fn is_root(node: &BranchNode) -> bool {
    node.parent.is_none()
}

pub fn is_stale(node: &BranchNode, now_unix_s: u64) -> bool {
    match node.last_commit_unix_s {
        Some(ts) => now_unix_s.saturating_sub(ts) > BRANCH_STALE_AFTER_SECS,
        None => true,
    }
}

/// Pure port for branch observation.
pub trait BranchObservationPort {
    fn observe(&self) -> Result<BranchGraph, String>;
}
