use serde::{Deserialize, Serialize};

pub const BRANCH_GRAPH_SCHEMA: &str = "cynic.branch_graph.v1";

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
    pub parent: String,
    pub has_upstream: bool,
    pub is_stale: bool,
}

pub fn is_root(node: &BranchNode) -> bool {
    node.parent.is_empty()
}
