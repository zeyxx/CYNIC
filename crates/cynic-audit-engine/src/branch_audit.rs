//! Branch triage audit — decides whether a branch still speaks about the
//! current organism, by measurement instead of by age or by commit count.

use std::path::Path;
use serde::Serialize;

use crate::ports::GitObserverPort;

/// Above this share of vanished files, a branch is about a world that no
/// longer exists. Not a tolerance: a measurement with a stated threshold.
const DISAPPEARED_RATIO_BLOCKS: f64 = 0.50;

/// Below this many files the ratio is noise, not a verdict.
const MIN_FILES_FOR_RATIO: usize = 4;

/// `archive/` is a state, not a verdict: those refs are parked deliberately and
/// must not be reported as blocked work waiting for a decision.
const ARCHIVE_PREFIX: &str = "archive/";

#[derive(Debug, Clone, Serialize)]
pub struct BranchAuditReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub rule: &'static str,
    pub base_ref: String,
    pub branch_count: usize,
    pub ok_count: usize,
    pub blocked_count: usize,
    pub archived_count: usize,
    pub branches: Vec<BranchAuditEntry>,
    pub worktree: WorktreeObservation,
}

#[derive(Debug, Clone, Serialize)]
pub struct BranchAuditEntry {
    pub branch: String,
    pub status: &'static str,
    pub unique_commits: usize,
    pub touched_files: usize,
    pub vanished_files: usize,
    pub vanished_ratio: f64,
    pub duplicate_patches: usize,
    pub last_commit_age: String,
    pub checks: Vec<BranchAuditCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BranchAuditCheck {
    pub name: &'static str,
    pub status: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorktreeObservation {
    pub schema: &'static str,
    pub status: &'static str,
    pub rule: &'static str,
    pub current_branch: String,
    pub head: String,
    pub dirty_path_count: usize,
    pub untracked_count: usize,
    pub live_claim_count: usize,
    pub unowned_paths: Vec<String>,
    pub uncorroborated_claims: Vec<String>,
    pub checks: Vec<BranchAuditCheck>,
}

/// Refuse to guess: an unreadable git surface is an unknown, never "ok".
pub fn audit_branches(
    git: &dyn GitObserverPort,
    workspace_root: &Path,
    base_ref: &str,
) -> BranchAuditReport {
    let mut branches = Vec::new();
    for branch in local_branches(git, base_ref) {
        branches.push(audit_branch(git, workspace_root, base_ref, &branch));
    }
    let total = branches.len();
    let archived = branches
        .iter()
        .filter(|entry| entry.status == "archived")
        .count();
    let blocked = branches
        .iter()
        .filter(|entry| entry.status == "blocked")
        .count();
    BranchAuditReport {
        schema: "cynic.cli.branch_audit.v1",
        status: if blocked == 0 { "ok" } else { "blocked" },
        rule: "a branch whose file set has largely vanished from the base describes a past, not a proposal; a branch whose patches already landed by another route carries nothing new. Classification only: merge and delete authority stays with the operator.",
        base_ref: base_ref.to_string(),
        branch_count: total,
        ok_count: total.saturating_sub(blocked).saturating_sub(archived),
        blocked_count: blocked,
        archived_count: archived,
        branches,
        worktree: observe_worktree(git, workspace_root),
    }
}

/// Read the local machine's real activity: what the worktree changed, and
/// whether a live claim covers each change.
fn observe_worktree(git: &dyn GitObserverPort, workspace_root: &Path) -> WorktreeObservation {
    let now_s = now_unix_s();
    let status_out = git.run_git(&["status", "--porcelain"]);
    let mut dirty: Vec<(String, bool)> = Vec::new(); // (path, untracked)
    for line in status_out.stdout.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (code, path) = line.split_at(2);
        let path = path.trim();
        if path.is_empty() {
            continue;
        }
        dirty.push((path.to_string(), code.starts_with("??")));
    }
    let untracked_count = dirty.iter().filter(|(_, untracked)| *untracked).count();

    let claims = live_claim_paths(workspace_root, now_s);
    let unowned: Vec<String> = dirty
        .iter()
        .map(|(path, _)| path.clone())
        .filter(|path| !claims.iter().any(|claim| claim_covers(claim, path)))
        .collect();

    let uncorroborated: Vec<String> = claims
        .iter()
        .filter(|claim| !dirty.iter().any(|(path, _)| claim_covers(claim, path)))
        .cloned()
        .collect();

    let mut checks = Vec::new();
    checks.push(BranchAuditCheck {
        name: "mutation_has_mandate",
        status: if unowned.is_empty() { "ok" } else { "blocked" },
        detail: format!(
            "{} dirty path(s), {} not covered by any live claim: {:?}",
            dirty.len(),
            unowned.len(),
            unowned
        ),
    });
    checks.push(BranchAuditCheck {
        name: "claim_is_corroborated",
        status: if uncorroborated.is_empty() {
            "ok"
        } else {
            "observe"
        },
        detail: format!(
            "{} live claim(s) hold a scope no local change touches — a claim may precede work, so this is an observation, not a fault",
            uncorroborated.len()
        ),
    });

    let status = if unowned.is_empty() { "ok" } else { "blocked" };
    let cur_branch = git.run_git(&["rev-parse", "--abbrev-ref", "HEAD"]);
    let head = git.run_git(&["rev-parse", "--short", "HEAD"]);
    WorktreeObservation {
        schema: "cynic.cli.worktree_observation.v1",
        status,
        rule: "a session at work is proven by what it changed, not by what it claimed; a mutated path with no live claim is mutation without a mandate",
        current_branch: cur_branch.stdout.trim().to_string(),
        head: head.stdout.trim().to_string(),
        dirty_path_count: dirty.len(),
        untracked_count,
        live_claim_count: claims.len(),
        unowned_paths: unowned,
        uncorroborated_claims: uncorroborated,
        checks,
    }
}

fn live_claim_paths(workspace_root: &Path, now_s: u64) -> Vec<String> {
    let Ok(raw) = std::fs::read_to_string(workspace_root.join(".agent-claims.json")) else {
        return Vec::new();
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    doc.get("claims")
        .and_then(|claims| claims.as_array())
        .map(|claims| {
            claims
                .iter()
                .filter(|claim| is_claim_active(claim, now_s))
                .filter_map(|claim| {
                    claim
                        .get("scope")
                        .and_then(|scope| scope.get("path"))
                        .and_then(|path| path.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn is_claim_active(claim: &serde_json::Value, now_s: u64) -> bool {
    if claim.get("status").and_then(|v| v.as_str()).unwrap_or("") != "active" {
        return false;
    }
    let started = claim
        .get("temporal")
        .and_then(|t| t.get("started_at_unix_s"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let ttl = claim
        .get("temporal")
        .and_then(|t| t.get("ttl_seconds"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1800);
    now_s < started.saturating_add(ttl)
}

fn claim_covers(claim_scope: &str, path: &str) -> bool {
    claim_scope == path || path.starts_with(&format!("{claim_scope}/"))
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn audit_branch(
    git: &dyn GitObserverPort,
    workspace_root: &Path,
    base_ref: &str,
    branch: &str,
) -> BranchAuditEntry {
    let rev_list = git.run_git(&[
        "rev-list",
        "--count",
        &format!("{base_ref}..{branch}"),
    ]);
    let unique_commits = count_lines(&rev_list.stdout);

    let diff_out = git.run_git(&["diff", "--name-only", &format!("{base_ref}...{branch}")]);
    let files: Vec<String> = diff_out
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();

    let vanished = files
        .iter()
        .filter(|file| !workspace_root.join(file).exists())
        .count();
    let touched = files.len();
    let ratio = if touched == 0 {
        0.0
    } else {
        vanished as f64 / touched as f64
    };

    let duplicates = count_duplicate_patches(git, base_ref, branch);
    let mut checks = Vec::new();

    let world_status = if touched < MIN_FILES_FOR_RATIO {
        "observe"
    } else if ratio > DISAPPEARED_RATIO_BLOCKS {
        "blocked"
    } else {
        "ok"
    };
    checks.push(BranchAuditCheck {
        name: "world_survival",
        status: world_status,
        detail: format!(
            "{vanished}/{touched} files touched by this branch no longer exist in {base_ref} ({:.0}%); ratio blocks above {:.0}%",
            ratio * 100.0,
            DISAPPEARED_RATIO_BLOCKS * 100.0
        ),
    });

    let novelty_status = if unique_commits > 0 && duplicates == unique_commits {
        "blocked"
    } else if duplicates > 0 {
        "observe"
    } else {
        "ok"
    };
    checks.push(BranchAuditCheck {
        name: "patch_novelty",
        status: novelty_status,
        detail: format!(
            "{duplicates}/{} unmerged commits carry a patch-id already present in {base_ref}; commit count alone says nothing about novelty",
            unique_commits
        ),
    });

    let status = if branch.starts_with(ARCHIVE_PREFIX) {
        "archived"
    } else if checks.iter().any(|check| check.status == "blocked") {
        "blocked"
    } else {
        "ok"
    };
    let last_age = git.run_git(&["log", "-1", "--format=%ar", branch]);
    BranchAuditEntry {
        branch: branch.to_string(),
        status,
        unique_commits,
        touched_files: touched,
        vanished_files: vanished,
        vanished_ratio: ratio,
        duplicate_patches: duplicates,
        last_commit_age: last_age.stdout.trim().to_string(),
        checks,
    }
}

fn local_branches(git: &dyn GitObserverPort, base_ref: &str) -> Vec<String> {
    let out = git.run_git(&["for-each-ref", "--format=%(refname:short)", "refs/heads"]);
    out.stdout
        .lines()
        .map(str::trim)
        .filter(|branch| !branch.is_empty())
        .map(str::to_string)
        .filter(|branch| branch != base_ref)
        .filter(|branch| {
            let rev_cnt = git.run_git(&[
                "rev-list",
                "--count",
                &format!("{base_ref}..{branch}"),
            ]);
            count_lines(&rev_cnt.stdout) > 0
        })
        .collect()
}

fn count_duplicate_patches(git: &dyn GitObserverPort, base_ref: &str, branch: &str) -> usize {
    let base_patches = base_patch_ids(git, base_ref);
    if base_patches.is_empty() {
        return 0;
    }
    let commits = git.run_git(&["rev-list", &format!("{base_ref}..{branch}"), "--no-merges"]);
    let mut duplicates = 0usize;
    for commit in commits
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Some(patch) = git.patch_id(commit) else {
            continue;
        };
        if base_patches.contains(&patch) {
            duplicates += 1;
        }
    }
    duplicates
}

fn base_patch_ids(git: &dyn GitObserverPort, base_ref: &str) -> Vec<String> {
    let commits = git.run_git(&["rev-list", base_ref, "--no-merges"]);
    let mut ids = Vec::new();
    for commit in commits
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if let Some(patch) = git.patch_id(commit) {
            ids.push(patch);
        }
    }
    ids
}

fn count_lines(text: &str) -> usize {
    text.lines().filter(|line| !line.trim().is_empty()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_are_stated_not_implicit() {
        assert!(DISAPPEARED_RATIO_BLOCKS > 0.0 && DISAPPEARED_RATIO_BLOCKS < 1.0);
        assert!(MIN_FILES_FOR_RATIO >= 2);
    }

    #[test]
    fn line_counting_ignores_blank_lines() {
        assert_eq!(count_lines("a\n\nb\n"), 2);
        assert_eq!(count_lines(""), 0);
        assert_eq!(count_lines("\n\n"), 0);
    }
}
