//! Git topology observer — read-only git plumbing, no mutation.
//!
//! Produces `crate::git_topology::GitTopologyObservation` from the
//! local repository. All commands are bounded, timeout-free, and report
//! unknowns instead of fabricating numbers.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_process::{ArgvRunner, LocalArgvRunner};

use crate::git_topology::{
    GIT_TOPOLOGY_SCHEMA, GitBranchEntry, GitBranchState, GitTopologyObservation,
    GitTopologyObservationUnknown, GitTopologyUnknown,
};

const MAX_BRANCHES: usize = 256;

fn run_git(root: &Path, runner: &dyn ArgvRunner, args: &[&str]) -> Option<String> {
    let root_str = root.to_str()?;
    let mut argv: Vec<&str> = Vec::with_capacity(args.len() + 3);
    argv.push("git");
    argv.push("-C");
    argv.push(root_str);
    argv.extend_from_slice(args);
    let out = runner.run(&argv);
    if out.spawn_error.is_some() {
        return None;
    }
    if out.exit_code != Some(0) {
        return None;
    }
    Some(out.stdout)
}

pub fn is_inside_repo(root: &Path) -> bool {
    let runner = LocalArgvRunner;
    run_git(root, &runner, &["rev-parse", "--is-inside-work-tree"])
        .is_some_and(|s| s.trim() == "true")
}

pub fn current_branch(root: &Path) -> Option<String> {
    let runner = LocalArgvRunner;
    let s = run_git(root, &runner, &["branch", "--show-current"])?;
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn node_id_from_env() -> String {
    std::env::var("CYNIC_NODE_ID")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "cortex".to_string())
}

fn read_branch_refs(root: &Path, runner: &dyn ArgvRunner) -> Vec<(String, String, Option<u64>)> {
    let Some(out) = run_git(
        root,
        runner,
        &[
            "for-each-ref",
            "--format=%(refname:short)|%(objectname)|%(committerdate:unix)",
            "refs/heads/",
        ],
    ) else {
        return Vec::new();
    };
    let mut v = Vec::new();
    for line in out.lines().take(MAX_BRANCHES) {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '|');
        let name = parts.next().unwrap_or("").trim().to_string();
        if name.is_empty() {
            continue;
        }
        let sha = parts.next().unwrap_or("").trim().to_string();
        let ts = parts.next().and_then(|s| s.trim().parse::<u64>().ok());
        v.push((name, sha, ts));
    }
    v
}

fn read_first_parents(
    root: &Path,
    runner: &dyn ArgvRunner,
) -> std::collections::HashMap<String, String> {
    const MAX_COMMITS: usize = 20_000;
    let mut map = std::collections::HashMap::new();
    let Some(out) = run_git(root, runner, &["log", "--all", "--pretty=format:%H %P"]) else {
        return map;
    };
    for line in out.lines().take(MAX_COMMITS) {
        let mut parts = line.split_whitespace();
        let (Some(commit), Some(parent)) = (parts.next(), parts.next()) else {
            continue;
        };
        if commit.len() < 7 || parent.len() < 7 {
            continue;
        }
        map.entry(commit.to_string())
            .or_insert_with(|| parent.to_string());
    }
    map
}

fn has_remote_branch(root: &Path, runner: &dyn ArgvRunner, branch: &str) -> bool {
    let spec = format!("origin/{branch}");
    run_git(root, runner, &["rev-parse", "--verify", &spec]).is_some()
}

fn count_ahead_behind(
    root: &Path,
    runner: &dyn ArgvRunner,
    branch: &str,
) -> (Option<u32>, Option<u32>) {
    let ahead = run_git(
        root,
        runner,
        &["rev-list", "--count", &format!("origin/main..{branch}")],
    )
    .and_then(|s| s.trim().parse::<u32>().ok());
    let behind = run_git(
        root,
        runner,
        &["rev-list", "--count", &format!("{branch}..origin/main")],
    )
    .and_then(|s| s.trim().parse::<u32>().ok());
    (ahead, behind)
}

fn has_worktree(root: &Path, runner: &dyn ArgvRunner, branch: &str) -> Option<bool> {
    let out = run_git(root, runner, &["worktree", "list", "--porcelain"])?;
    let needle = format!("branch refs/heads/{branch}");
    Some(out.lines().any(|l| l.trim() == needle))
}

fn dirty_count_for_current(
    root: &Path,
    runner: &dyn ArgvRunner,
    is_current: bool,
) -> (Option<u32>, bool) {
    if !is_current {
        return (None, true); // unknown for non-current
    }
    let Some(out) = run_git(root, runner, &["status", "--porcelain"]) else {
        return (None, true);
    };
    let count = out.lines().filter(|l| !l.trim().is_empty()).count() as u32;
    (Some(count), false)
}

fn derive_state(behind: Option<u32>, _ahead: Option<u32>, has_remote: bool) -> GitBranchState {
    if !has_remote {
        return GitBranchState::Orphan;
    }
    match behind {
        Some(b) if b >= 100 => GitBranchState::Dead,
        Some(b) if b >= 50 => GitBranchState::Divergent,
        Some(b) if b <= 5 => GitBranchState::Stable,
        _ => GitBranchState::Active,
    }
}

pub fn observe_git_topology_with_runner(
    root: &Path,
    runner: &impl ArgvRunner,
    now_ms: u64,
    node_id: Option<&str>,
) -> GitTopologyObservation {
    let now = (now_ms / 1000) as u64;
    let resolved_node_id = node_id
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(node_id_from_env);

    let runner_dyn: &dyn ArgvRunner = runner;

    if !is_inside_repo_with_runner(root, runner_dyn) {
        return GitTopologyObservation {
            schema: GIT_TOPOLOGY_SCHEMA.to_string(),
            observed_at_unix_s: now,
            node_id: resolved_node_id,
            current_branch: None,
            entries: vec![],
            observation_unknowns: vec![GitTopologyObservationUnknown::GitCommandFailed],
        };
    }

    let current = current_branch_with_runner(root, runner_dyn);
    let refs = read_branch_refs(root, runner_dyn);

    if refs.is_empty() {
        return GitTopologyObservation {
            schema: GIT_TOPOLOGY_SCHEMA.to_string(),
            observed_at_unix_s: now,
            node_id: resolved_node_id,
            current_branch: current,
            entries: vec![],
            observation_unknowns: vec![GitTopologyObservationUnknown::EmptyOutput],
        };
    }

    let mut entries = Vec::new();
    let mut truncated = false;

    let tips: std::collections::HashMap<String, String> = refs
        .iter()
        .filter(|(_, sha, _)| !sha.is_empty())
        .map(|(b, sha, _)| (b.clone(), sha.clone()))
        .collect();
    let main_tip = run_git(root, runner_dyn, &["rev-parse", "main"])
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let stack_parents =
        crate::stack::infer_stack_parents(&tips, &read_first_parents(root, runner_dyn), &main_tip);

    for (branch, _sha, ts) in refs.into_iter().take(MAX_BRANCHES) {
        if entries.len() >= MAX_BRANCHES {
            truncated = true;
            break;
        }
        let has_remote = has_remote_branch(root, runner_dyn, &branch);
        let (ahead, behind) = if has_remote {
            count_ahead_behind(root, runner_dyn, &branch)
        } else {
            (None, None)
        };
        let is_current = current.as_deref() == Some(branch.as_str());
        let has_wt = has_worktree(root, runner_dyn, &branch);
        let (dirty, dirty_unknown) = dirty_count_for_current(root, runner_dyn, is_current);

        let mut unknowns = Vec::new();
        if ahead.is_none() {
            unknowns.push(GitTopologyUnknown::Ahead);
        }
        if behind.is_none() {
            unknowns.push(GitTopologyUnknown::Behind);
        }
        if ts.is_none() {
            unknowns.push(GitTopologyUnknown::LastCommit);
        }
        if has_wt.is_none() {
            unknowns.push(GitTopologyUnknown::Worktree);
        }
        if dirty_unknown {
            unknowns.push(GitTopologyUnknown::Dirty);
        }
        let parent = stack_parents.get(&branch).cloned();
        if parent.is_none() {
            unknowns.push(GitTopologyUnknown::Parent);
        }

        entries.push(GitBranchEntry {
            branch: branch.clone(),
            ahead,
            behind,
            last_commit_unix_s: ts,
            state: derive_state(behind, ahead, has_remote),
            has_worktree: has_wt,
            dirty_count: dirty,
            has_remote: Some(has_remote),
            parent,
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
        node_id: resolved_node_id,
        current_branch: current,
        entries,
        observation_unknowns: obs_unknowns,
    }
}

pub fn observe_git_topology(root: &Path) -> GitTopologyObservation {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let nid = node_id_from_env();
    observe_git_topology_with_runner(root, &LocalArgvRunner, now_ms, Some(&nid))
}

fn is_inside_repo_with_runner(root: &Path, runner: &dyn ArgvRunner) -> bool {
    run_git(root, runner, &["rev-parse", "--is-inside-work-tree"])
        .is_some_and(|s| s.trim() == "true")
}

fn current_branch_with_runner(root: &Path, runner: &dyn ArgvRunner) -> Option<String> {
    let s = run_git(root, runner, &["branch", "--show-current"])?;
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

// Hygiene + others unchanged below
pub fn observe_git_hygiene(root: &Path) -> crate::git_hygiene::GitHygieneObservation {
    use crate::git_hygiene::GIT_HYGIENE_SCHEMA;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let runner = LocalArgvRunner;
    let current = current_branch(root);

    let staged = run_git(root, &runner, &["diff", "--cached", "--name-only"])
        .map(|s| {
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let dirty = run_git(root, &runner, &["status", "--porcelain"])
        .map(|s| {
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let (claim_branch, claim_scope) = read_claim_scope(root);

    crate::git_hygiene::GitHygieneObservation {
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
    let Some(content) = content else {
        return (None, None);
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    let claims = json.get("claims").and_then(|v| v.as_array());
    let Some(claims) = claims else {
        return (None, None);
    };
    for claim in claims {
        if claim.get("status").and_then(|v| v.as_str()) != Some("active") {
            continue;
        }
        let scope = claim.get("scope");
        let kind = scope.and_then(|s| s.get("kind")).and_then(|v| v.as_str());
        let path = scope.and_then(|s| s.get("path")).and_then(|v| v.as_str());
        if let Some(p) = path {
            return (Some(p.to_string()), Some(p.to_string()));
        }
        if kind.is_some() {
            return (Some(kind.unwrap_or("").to_string()), None);
        }
    }
    (None, None)
}

pub fn observe_head_sha(root: &Path) -> Option<String> {
    let runner = LocalArgvRunner;
    let sha = run_git(root, &runner, &["rev-parse", "HEAD"])?;
    let trimmed = sha.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn count_dirty_files(root: &Path) -> usize {
    let runner = LocalArgvRunner;
    run_git(root, &runner, &["status", "--short"])
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_topology::validate_git_topology_observation;

    #[test]
    fn observes_this_repo() {
        let obs = observe_git_topology(Path::new("."));
        assert!(
            !obs.observation_unknowns
                .contains(&GitTopologyObservationUnknown::GitCommandFailed),
            "should be inside git repo"
        );
        assert!(
            validate_git_topology_observation(&obs).is_empty(),
            "validation: {:?}",
            validate_git_topology_observation(&obs)
        );
        assert!(
            obs.entries.iter().any(|e| e.branch == "main"),
            "entries: {:?}",
            obs.entries.iter().map(|e| &e.branch).collect::<Vec<_>>()
        );
    }
}
