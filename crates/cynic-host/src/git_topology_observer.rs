//! Git topology observer — read-only git plumbing, no mutation.
//!
//! Produces `cynic_protocol::git_topology::GitTopologyObservation` from the
//! local repository. All commands are bounded, timeout-free, and report
//! unknowns instead of fabricating numbers.

use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::git_topology::{
    GitBranchEntry, GitBranchState, GitTopologyObservation, GitTopologyUnknown,
    GitTopologyObservationUnknown, GIT_TOPOLOGY_SCHEMA,
};

const MAX_BRANCHES: usize = 256;

fn run_git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

fn is_inside_repo(root: &Path) -> bool {
    run_git(root, &["rev-parse", "--is-inside-work-tree"])
        .is_some_and(|s| s.trim() == "true")
}

fn current_branch(root: &Path) -> Option<String> {
    let s = run_git(root, &["branch", "--show-current"])?;
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

fn node_id() -> String {
    std::env::var("CYNIC_NODE_ID")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "cortex".to_string())
}

/// Parse `git for-each-ref` output into (branch, last_commit_unix_s)
fn read_branch_refs(root: &Path) -> Vec<(String, Option<u64>)> {
    let Some(out) = run_git(
        root,
        &[
            "for-each-ref",
            "--format=%(refname:short)|%(committerdate:unix)",
            "refs/heads/",
        ],
    ) else {
        return Vec::new();
    };
    let mut v = Vec::new();
    for line in out.lines().take(MAX_BRANCHES) {
        if line.trim().is_empty() { continue; }
        let mut parts = line.splitn(2, '|');
        let name = parts.next().unwrap_or("").trim().to_string();
        if name.is_empty() { continue; }
        let ts = parts.next().and_then(|s| s.trim().parse::<u64>().ok());
        v.push((name, ts));
    }
    v
}

fn has_remote_branch(root: &Path, branch: &str) -> bool {
    // Check origin/<branch> exists
    Command::new("git")
        .args(["rev-parse", "--verify", &format!("origin/{branch}")])
        .current_dir(root)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn count_ahead_behind(root: &Path, branch: &str) -> (Option<u32>, Option<u32>) {
    // Ahead = commits in branch not in origin/main
    // Behind = commits in origin/main not in branch
    let ahead = run_git(root, &["rev-list", "--count", &format!("origin/main..{branch}")])
        .and_then(|s| s.trim().parse::<u32>().ok());
    let behind = run_git(root, &["rev-list", "--count", &format!("{branch}..origin/main")])
        .and_then(|s| s.trim().parse::<u32>().ok());
    (ahead, behind)
}

fn has_worktree(root: &Path, branch: &str) -> Option<bool> {
    let out = run_git(root, &["worktree", "list", "--porcelain"])?;
    // porcelain lists "branch refs/heads/<name>" per worktree
    let needle = format!("branch refs/heads/{branch}");
    Some(out.lines().any(|l| l.trim() == needle))
}

fn dirty_count_for_current(root: &Path, is_current: bool) -> (Option<u32>, bool) {
    if !is_current {
        return (None, true); // unknown for non-current
    }
    let Some(out) = run_git(root, &["status", "--porcelain"]) else {
        return (None, true);
    };
    let count = out.lines().filter(|l| !l.trim().is_empty()).count() as u32;
    (Some(count), false)
}

fn derive_state(behind: Option<u32>, _ahead: Option<u32>, has_remote: bool) -> GitBranchState {
    if !has_remote { return GitBranchState::Orphan; }
    match behind {
        Some(b) if b >= 100 => GitBranchState::Dead,
        Some(b) if b >= 50 => GitBranchState::Divergent,
        Some(b) if b <= 5 => GitBranchState::Stable,
        _ => GitBranchState::Active,
    }
}

pub fn observe_git_topology(root: &Path) -> GitTopologyObservation {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

    if !is_inside_repo(root) {
        return GitTopologyObservation {
            schema: GIT_TOPOLOGY_SCHEMA.to_string(),
            observed_at_unix_s: now,
            node_id: node_id(),
            current_branch: None,
            entries: vec![],
            observation_unknowns: vec![GitTopologyObservationUnknown::GitCommandFailed],
        };
    }

    let current = current_branch(root);
    let refs = read_branch_refs(root);

    if refs.is_empty() {
        return GitTopologyObservation {
            schema: GIT_TOPOLOGY_SCHEMA.to_string(),
            observed_at_unix_s: now,
            node_id: node_id(),
            current_branch: current,
            entries: vec![],
            observation_unknowns: vec![GitTopologyObservationUnknown::EmptyOutput],
        };
    }

    let mut entries = Vec::new();
    let mut truncated = false;

    for (branch, ts) in refs.into_iter().take(MAX_BRANCHES) {
        if entries.len() >= MAX_BRANCHES {
            truncated = true;
            break;
        }
        let has_remote = has_remote_branch(root, &branch);
        let (ahead, behind) = if has_remote { count_ahead_behind(root, &branch) } else { (None, None) };
        let is_current = current.as_deref() == Some(branch.as_str());
        let has_wt = has_worktree(root, &branch);
        let (dirty, dirty_unknown) = dirty_count_for_current(root, is_current);

        let mut unknowns = Vec::new();
        if ahead.is_none() { unknowns.push(GitTopologyUnknown::Ahead); }
        if behind.is_none() { unknowns.push(GitTopologyUnknown::Behind); }
        if ts.is_none() { unknowns.push(GitTopologyUnknown::LastCommit); }
        if has_wt.is_none() { unknowns.push(GitTopologyUnknown::Worktree); }
        if dirty_unknown { unknowns.push(GitTopologyUnknown::Dirty); }

        entries.push(GitBranchEntry {
            branch: branch.clone(),
            ahead,
            behind,
            last_commit_unix_s: ts,
            state: derive_state(behind, ahead, has_remote),
            has_worktree: has_wt,
            dirty_count: dirty,
            has_remote: Some(has_remote),
            unknowns,
        });
    }

    let mut obs_unknowns = Vec::new();
    if truncated {
        obs_unknowns.push(GitTopologyObservationUnknown::OutputLimitExceeded);
    }
    if entries.len() >= MAX_BRANCHES {
        obs_unknowns.push(GitTopologyObservationUnknown::BranchLimitExceeded);
    }

    GitTopologyObservation {
        schema: GIT_TOPOLOGY_SCHEMA.to_string(),
        observed_at_unix_s: now,
        node_id: node_id(),
        current_branch: current,
        entries,
        observation_unknowns: obs_unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::git_topology::validate_git_topology_observation;

    #[test]
    fn observes_this_repo() {
        let obs = observe_git_topology(Path::new("."));
        // Should not be GitCommandFailed when run inside repo
        assert!(
            !obs.observation_unknowns.contains(&GitTopologyObservationUnknown::GitCommandFailed),
            "should be inside git repo"
        );
        assert!(validate_git_topology_observation(&obs).is_empty(), "validation: {:?}", validate_git_topology_observation(&obs));
        // Must contain at least main
        assert!(obs.entries.iter().any(|e| e.branch == "main"), "entries: {:?}", obs.entries.iter().map(|e| &e.branch).collect::<Vec<_>>());
    }
}

// ---------------------------------------------------------------------------
// Hygiene — staged vs claim coherence
// ---------------------------------------------------------------------------

pub fn observe_git_hygiene(root: &Path) -> cynic_protocol::git_hygiene::GitHygieneObservation {
    use cynic_protocol::git_hygiene::GIT_HYGIENE_SCHEMA;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let current = current_branch(root);

    let staged = run_git(root, &["diff", "--cached", "--name-only"])
        .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect::<Vec<_>>())
        .unwrap_or_default();

    let dirty = run_git(root, &["status", "--porcelain"])
        .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect::<Vec<_>>())
        .unwrap_or_default();

    // Read claim scope from .agent-claims.json (best-effort)
    let (claim_branch, claim_scope) = read_claim_scope(root);

    cynic_protocol::git_hygiene::GitHygieneObservation {
        schema: GIT_HYGIENE_SCHEMA.to_string(),
        observed_at_unix_s: now,
        current_branch: current,
        staged_files: staged,
        dirty_files: dirty,
        claim_branch,
        claim_scope,
        unknowns: vec![],
    }
}

fn read_claim_scope(root: &Path) -> (Option<String>, Option<String>) {
    let path = root.join(".agent-claims.json");
    let content = std::fs::read_to_string(&path).ok();
    let Some(content) = content else { return (None, None); };
    let json: serde_json::Value = match serde_json::from_str(&content) { Ok(v) => v, Err(_) => return (None, None) };
    let claims = json.get("claims").and_then(|v| v.as_array());
    let Some(claims) = claims else { return (None, None); };
    for claim in claims {
        if claim.get("status").and_then(|v| v.as_str()) != Some("active") { continue; }
        let scope = claim.get("scope");
        let kind = scope.and_then(|s| s.get("kind")).and_then(|v| v.as_str());
        let path = scope.and_then(|s| s.get("path")).and_then(|v| v.as_str());
        if let Some(p) = path {
            // Map claim scope path to a prefix for hygiene (crate/file)
            return (Some(p.to_string()), Some(p.to_string()));
        }
        if kind.is_some() {
            return (Some(kind.unwrap_or("").to_string()), None);
        }
    }
    (None, None)
}
