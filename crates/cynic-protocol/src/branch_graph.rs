use std::collections::HashMap;

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

/// Convenience: check if a node is stale via policy.
pub fn stale_via_policy(node: &BranchNode, now: u64) -> bool {
    is_stale(node, now)
}

/// Pure port for branch observation.
pub trait BranchObservationPort {
    fn observe(&self) -> Result<BranchGraph, String>;
}

/// Parse `git for-each-ref` raw output into a `BranchGraph`.
///
/// Each line: `<name>|<commit>|<upstream>|<unix_ts>`
/// - upstream may be empty (no upstream)
/// - unix_ts may be empty (unknown)
/// Pure, no FS/git I/O. `is_stale` is derived via policy (`is_stale()`), not stored.
pub fn parse_git_for_each_ref(raw: &str, root_commit: &str, _now: u64) -> BranchGraph {
    let mut nodes: Vec<BranchNode> = Vec::new();
    // Optional parent map enrichment happens via `parse_git_log_graph` caller-side;
    // pure here => parent is None initially.
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[0].trim().to_string();
        if name.is_empty() {
            continue;
        }
        let commit = parts[1].trim().to_string();
        if commit.is_empty() {
            continue;
        }
        let upstream = parts.get(2).map(|s| s.trim()).unwrap_or("");
        let has_upstream = !upstream.is_empty();
        let last_commit_unix_s: Option<u64> = parts.get(3).and_then(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                s.parse::<u64>().ok()
            }
        });

        nodes.push(BranchNode {
            name,
            commit,
            parent: None,
            has_upstream,
            last_commit_unix_s,
        });
    }

    // Sort for deterministic output.
    nodes.sort_by(|a, b| a.name.cmp(&b.name));

    // Root = branch whose commit == root_commit, else first node, else "main".
    let root = nodes
        .iter()
        .find(|n| n.commit == root_commit)
        .map(|n| n.name.clone())
        .or_else(|| nodes.first().map(|n| n.name.clone()))
        .unwrap_or_else(|| "main".to_string());

    BranchGraph {
        schema: BRANCH_GRAPH_SCHEMA.to_string(),
        root,
        root_commit: root_commit.to_string(),
        nodes,
    }
}

/// Parse `git log --all --pretty=format:%H %P` raw stdout into commit -> first parent.
///
/// Each line: `<commit> [parent1 [parent2 ...]]`
/// Lines with no parent map to None (root commits). Graph decorations (`*`, `|`)
/// are stripped if present (supports `git log --graph` fallback).
/// Pure, no I/O.
pub fn parse_git_log_graph(raw: &str) -> HashMap<String, Option<String>> {
    let mut map = HashMap::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Strip leading graph chars `*`, `|`, `/`, `\`, spaces.
        let cleaned = line
            .trim_start_matches(|c: char| c == '*' || c == '|' || c == '/' || c == '\\' || c == ' ')
            .trim();
        if cleaned.is_empty() {
            continue;
        }
        let mut parts = cleaned.split_whitespace();
        let Some(commit) = parts.next() else {
            continue;
        };
        // Validate hex commit (at least 7 chars hex) to skip decoration noise.
        if !commit.chars().all(|c| c.is_ascii_hexdigit()) || commit.len() < 7 {
            continue;
        }
        let parent = parts.next().map(|p| p.to_string());
        // Only store if parent looks like a commit hash.
        let parent = parent.filter(|p| p.chars().all(|c| c.is_ascii_hexdigit()) && p.len() >= 7);
        map.insert(commit.to_string(), parent);
    }
    map
}

/// Enrich a BranchGraph's nodes with parent branch names using a commit->parent map.
/// Pure helper: maps parent commit -> branch name that points to it.
pub fn enrich_with_parents(
    mut graph: BranchGraph,
    commit_to_parent: &HashMap<String, Option<String>>,
) -> BranchGraph {
    // Build commit -> branch name lookup (first branch owning that commit).
    let commit_to_branch: HashMap<String, String> = graph
        .nodes
        .iter()
        .map(|n| (n.commit.clone(), n.name.clone()))
        .collect();

    for node in &mut graph.nodes {
        if let Some(Some(parent_commit)) = commit_to_parent.get(&node.commit) {
            if let Some(parent_branch) = commit_to_branch.get(parent_commit) {
                // Don't self-parent.
                if parent_branch != &node.name {
                    node.parent = Some(parent_branch.clone());
                }
            }
        }
    }
    graph
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_for_each_ref_basic() {
        let raw = "main|abc123def456abc123def456abc123def456abcd|origin/main|1700000000\nfeature|def456abc123def456abc123def456abc123def456ab||1690000000\nempty-ts|aaa111aaa111aaa111aaa111aaa111aaa111aaaa|origin/empty-ts|";
        let g = parse_git_for_each_ref(raw, "abc123def456abc123def456abc123def456abcd", 1700000000);
        assert_eq!(g.root, "main");
        assert_eq!(g.root_commit, "abc123def456abc123def456abc123def456abcd");
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.nodes[0].name, "empty-ts");
        assert_eq!(g.nodes[1].name, "feature");
        assert_eq!(g.nodes[2].name, "main");
        assert!(g.nodes[2].has_upstream);
        assert!(!g.nodes[1].has_upstream);
        assert_eq!(g.nodes[2].last_commit_unix_s, Some(1700000000));
        assert_eq!(g.nodes[0].last_commit_unix_s, None);
        assert!(is_stale(&g.nodes[0], 1700000000));
        assert!(is_stale(&g.nodes[1], 1700000000));
        assert!(!is_stale(&g.nodes[2], 1700000000));
    }

    #[test]
    fn parse_for_each_ref_empty() {
        let g = parse_git_for_each_ref("", "deadbeef", 0);
        assert_eq!(g.root, "main");
        assert!(g.nodes.is_empty());
    }

    #[test]
    fn parse_git_log_graph_basic() {
        let raw = "abc123def456abc123def456abc123def456abcd\ndef456abc123def456abc123def456abc123def456ab abc123def456abc123def456abc123def456abcd\n* aaa111aaa111aaa111aaa111aaa111aaa111aaaa def456abc123def456abc123def456abc123def456ab";
        let m = parse_git_log_graph(raw);
        assert_eq!(
            m.get("abc123def456abc123def456abc123def456abcd"),
            Some(&None)
        );
        assert_eq!(
            m.get("def456abc123def456abc123def456abc123def456ab"),
            Some(&Some(
                "abc123def456abc123def456abc123def456abcd".to_string()
            ))
        );
    }

    #[test]
    fn enrich_parents() {
        let raw = "main|aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa|origin/main|1700000000\nfeature|bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb||1700000000";
        let g = parse_git_for_each_ref(raw, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", 1700000000);
        let log = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let map = parse_git_log_graph(log);
        let enriched = enrich_with_parents(g, &map);
        let feat = enriched.nodes.iter().find(|n| n.name == "feature").unwrap();
        assert_eq!(feat.parent.as_deref(), Some("main"));
    }

    #[test]
    fn stale_policy_threshold() {
        let node = BranchNode {
            name: "x".into(),
            commit: "abc".into(),
            parent: None,
            has_upstream: false,
            last_commit_unix_s: Some(1000),
        };
        assert!(!stale_via_policy(&node, 1000 + BRANCH_STALE_AFTER_SECS));
        assert!(stale_via_policy(&node, 1000 + BRANCH_STALE_AFTER_SECS + 1));
    }
}
