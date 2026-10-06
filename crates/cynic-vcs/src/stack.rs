//! Pure stack-parent inference over the branch DAG (no I/O).
//!
//! The commit graph is a DAG; review order is a derived forest of stacks.
//! [`infer_stack_parents`] walks first-parents from each branch tip to the
//! nearest other branch tip (or main). A branch whose walk finds nothing
//! keeps `None` — an explicit unknown, never a guess. Cycles and missing
//! commits also yield `None`. Second parents of merges are ignored by
//! documented heuristic (first-parent walk); the DAG stays authoritative.

use std::collections::{HashMap, HashSet};

/// Maximum first-parent walk steps per branch (R4: bounded everything).
pub const MAX_STACK_WALK_STEPS: usize = 4096;

/// First-parent history set of main, for fork-point detection.
fn main_history(first_parent: &HashMap<String, String>, main_tip: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    let mut cursor = main_tip.to_string();
    for _ in 0..MAX_STACK_WALK_STEPS {
        if !set.insert(cursor.clone()) {
            break;
        }
        let Some(next) = first_parent.get(&cursor) else {
            break;
        };
        cursor = next.clone();
    }
    set
}

/// Infer `branch -> parent branch` over tip SHAs.
///
/// Walk each tip's first-parent chain: the nearest other branch tip hit
/// before the fork point is the stack parent; otherwise the branch forked
/// directly off main's history and the parent is "main". A branch tipping
/// exactly on main resolves to "main". Absent key = unknown (cycle,
/// missing commits, budget exhausted, or no fork point found).
pub fn infer_stack_parents(
    tips: &HashMap<String, String>,
    first_parent: &HashMap<String, String>,
    main_tip: &str,
) -> HashMap<String, String> {
    // Reverse index: commit SHA -> branch names tipping exactly there.
    let mut tip_at: HashMap<&str, Vec<&str>> = HashMap::new();
    for (branch, sha) in tips {
        tip_at
            .entry(sha.as_str())
            .or_default()
            .push(branch.as_str());
    }
    let main_hist = main_history(first_parent, main_tip);
    let mut out = HashMap::new();
    for (branch, sha) in tips {
        if branch == "main" {
            continue;
        }
        if let Some(parent) = walk_to_parent(sha, branch, &tip_at, first_parent, &main_hist) {
            out.insert(branch.clone(), parent);
        }
    }
    out
}

fn walk_to_parent(
    start_sha: &str,
    start_branch: &str,
    tip_at: &HashMap<&str, Vec<&str>>,
    first_parent: &HashMap<String, String>,
    main_hist: &HashSet<String>,
) -> Option<String> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut cursor = start_sha;
    // Skip the tip itself: a branch stacked exactly on another tip still
    // resolves to that tip's branch. A tip on main history resolves "main".
    let mut first = true;
    for _ in 0..MAX_STACK_WALK_STEPS {
        if !first {
            if let Some(branches) = tip_at.get(cursor) {
                // Nearest other branch tip on the walk = stack parent.
                // "main" excluded here (handled by history check below);
                // deterministic: smallest name wins ties.
                if let Some(other) = branches
                    .iter()
                    .filter(|b| **b != start_branch && **b != "main")
                    .min()
                {
                    return Some(other.to_string());
                }
            }
            if main_hist.contains(cursor) {
                return Some("main".to_string());
            }
        } else if main_hist.contains(cursor) {
            return Some("main".to_string());
        }
        first = false;
        if !seen.insert(cursor) {
            return None; // cycle guard
        }
        let next = first_parent.get(cursor)?;
        cursor = next.as_str();
    }
    None // walk budget exhausted
}

/// Stack depth of a branch following `parents` (branch -> parent).
/// `None` on cycle, missing link, or budget exhaustion. Bounded by entries+1.
pub fn stack_depth(branch: &str, parents: &HashMap<String, String>, max: usize) -> Option<u32> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut cursor = branch;
    let mut depth: u32 = 0;
    let bound = max.saturating_add(1).max(1);
    for _ in 0..bound {
        let next = parents.get(cursor)?;
        if next == "main" {
            return Some(depth.saturating_add(1));
        }
        if !seen.insert(cursor) {
            return None;
        }
        cursor = next.as_str();
        depth = depth.saturating_add(1);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn linear_stack_resolves_parents() {
        // main@c0 <- a@c1 <- b@c2
        let tips = map(&[("main", "c0"), ("a", "c1"), ("b", "c2")]);
        let fp = map(&[("c1", "c0"), ("c2", "c1")]);
        let parents = infer_stack_parents(&tips, &fp, "c0");
        assert_eq!(parents.get("a"), Some(&"main".to_string()));
        assert_eq!(parents.get("b"), Some(&"a".to_string()));
        assert!(!parents.contains_key("main"));
    }

    #[test]
    fn diverged_branch_forks_off_main_history() {
        // main: c0 <- c1 <- c2(tip); a branched at c1, now at c3
        let tips = map(&[("main", "c2"), ("a", "c3")]);
        let fp = map(&[("c1", "c0"), ("c2", "c1"), ("c3", "c1")]);
        let parents = infer_stack_parents(&tips, &fp, "c2");
        assert_eq!(parents.get("a"), Some(&"main".to_string()));
    }

    #[test]
    fn stacked_branch_prefers_nearest_tip_over_main() {
        // main: c0 <- c1; a tips at c1 (on main history); b stacked on a at c2
        let tips = map(&[("main", "c1"), ("a", "c1"), ("b", "c2")]);
        let fp = map(&[("c1", "c0"), ("c2", "c1")]);
        let parents = infer_stack_parents(&tips, &fp, "c1");
        assert_eq!(parents.get("a"), Some(&"main".to_string()));
        assert_eq!(parents.get("b"), Some(&"a".to_string()));
    }

    #[test]
    fn missing_history_yields_unknown_not_guess() {
        let tips = map(&[("main", "c0"), ("a", "c9")]);
        let fp = map(&[("c1", "c0")]); // c9 absent
        let parents = infer_stack_parents(&tips, &fp, "c0");
        assert!(!parents.contains_key("a"));
    }

    #[test]
    fn cycle_yields_unknown() {
        let tips = map(&[("main", "c0"), ("a", "c1")]);
        let fp = map(&[("c1", "c2"), ("c2", "c1")]);
        let parents = infer_stack_parents(&tips, &fp, "c0");
        assert!(!parents.contains_key("a"));
        assert_eq!(stack_depth("a", &map(&[("a", "b"), ("b", "a")]), 10), None);
    }

    #[test]
    fn stack_depth_counts_to_main() {
        let parents = map(&[("b", "a"), ("a", "main")]);
        assert_eq!(stack_depth("b", &parents, 10), Some(2));
        assert_eq!(stack_depth("a", &parents, 10), Some(1));
        assert_eq!(stack_depth("orphan", &parents, 10), None);
    }
}
