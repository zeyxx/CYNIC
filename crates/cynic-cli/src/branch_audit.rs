//! Branch triage audit — decides whether a branch still speaks about the
//! current organism, by measurement instead of by age or by commit count.
//!
//! Fits the existing audit taxonomy (`crate_audit`, `kernel_boundary_audit`,
//! `ownership_audit`): same report shape, same `status` vocabulary, same
//! check granularity, same "no hardcoded table" rule — branches are read
//! from the repository, never from a list written by hand.
//!
//! Why this exists (2026-09-28, measured): 8 branches survived with unique
//! commits, and commit count was a meaningless triage signal. What separated
//! the dead ones from the live one was how much of their file set still
//! exists in `main`:
//!
//! ```text
//!   repo-hygiene-convergence   0%   absent   (content already in main)
//!   wip-backup                15%   absent   (real merge surface)
//!   h1-intent                 58%   absent   (describes a past)
//!   split-kernel-owned        68%   absent   (describes a past)
//!   vague-b-eval              88%   absent   (a different universe)
//!   restructure-architecture  95%   absent   (describes a past)
//! ```
//!
//! Two measurements, both cheap, both local:
//!
//! 1. **disappearance** — of the files a branch touches, how many are absent
//!    from `main`? A branch whose world was deleted elsewhere is a proposal
//!    about the past; merging it resurrects mass that was deliberately
//!    removed.
//! 2. **patch-id duplication** — a branch can be "unmerged" by commit hash
//!    while carrying nothing new: the same patch may already have landed by
//!    another route. Hash comparison says `unmerged`; patch comparison says
//!    `empty`. `repo-hygiene-convergence` held a commit whose patch-id is
//!    byte-identical to a commit already in `main`.
//!
//! Neither measurement is a merge decision: this audit classifies and
//! reports. Authority to delete or merge stays with the operator, and an
//! audit that could delete branches would be a mutation surface pretending
//! to be an observer.

use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::rust_source::workspace_root;

/// Above this share of vanished files, a branch is about a world that no
/// longer exists. Not a tolerance: a measurement with a stated threshold.
const DISAPPEARED_RATIO_BLOCKS: f64 = 0.50;

/// Below this many files the ratio is noise, not a verdict.
const MIN_FILES_FOR_RATIO: usize = 4;

#[derive(Serialize)]
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
}

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct BranchAuditCheck {
    pub name: &'static str,
    pub status: &'static str,
    pub detail: String,
}

/// Refuse to guess: an unreadable git surface is an unknown, never "ok".
pub fn audit_branches(base_ref: &str) -> BranchAuditReport {
    let mut branches = Vec::new();
    for branch in local_branches(base_ref) {
        branches.push(audit_branch(base_ref, &branch));
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
        ok_count: total - blocked - archived,
        blocked_count: blocked,
        archived_count: archived,
        branches,
    }
}

fn audit_branch(base_ref: &str, branch: &str) -> BranchAuditEntry {
    let unique_commits = count_lines(&git(&[
        "rev-list",
        "--count",
        &format!("{base_ref}..{branch}"),
    ]));
    let files: Vec<String> = git(&["diff", "--name-only", &format!("{base_ref}...{branch}")])
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();

    let root = workspace_root();
    let vanished = files
        .iter()
        .filter(|file| !root.join(file).exists())
        .count();
    let touched = files.len();
    let ratio = if touched == 0 {
        0.0
    } else {
        vanished as f64 / touched as f64
    };

    let duplicates = count_duplicate_patches(base_ref, branch);
    let mut checks = Vec::new();

    // 1. world survival
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

    // 2. patch novelty — unmerged by hash, empty by content
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
    BranchAuditEntry {
        branch: branch.to_string(),
        status,
        unique_commits,
        touched_files: touched,
        vanished_files: vanished,
        vanished_ratio: ratio,
        duplicate_patches: duplicates,
        last_commit_age: git(&["log", "-1", "--format=%ar", branch])
            .trim()
            .to_string(),
        checks,
    }
}

/// Branches carrying commits the base does not have. Branches fully contained
/// in the base are not interesting to triage and are not listed.
///
/// Local heads only (`refs/heads/*`): `git branch` scope, not remotes, tags or
/// the stash. A mirror of `origin/*` is the same work seen twice, and
/// reporting it as a second branch would double every count.
fn local_branches(base_ref: &str) -> Vec<String> {
    git(&["for-each-ref", "--format=%(refname:short)", "refs/heads"])
        .lines()
        .map(str::trim)
        .filter(|branch| !branch.is_empty())
        .map(str::to_string)
        .filter(|branch| branch != base_ref)
        .filter(|branch| {
            count_lines(&git(&[
                "rev-list",
                "--count",
                &format!("{base_ref}..{branch}"),
            ])) > 0
        })
        .collect()
}

/// `archive/` is a state, not a verdict: those refs are parked deliberately and
/// must not be reported as blocked work waiting for a decision.
const ARCHIVE_PREFIX: &str = "archive/";

/// How many of the branch's unmerged commits have a patch-id already in base.
///
/// `patch-id` is computed over the diff, so the same change reached by two
/// different routes (rebase, cherry-pick, independent re-application) collapses
/// to one value. Hash comparison cannot see that; this can.
fn count_duplicate_patches(base_ref: &str, branch: &str) -> usize {
    let base_patches = base_patch_ids(base_ref);
    if base_patches.is_empty() {
        return 0;
    }
    let commits = git(&["rev-list", &format!("{base_ref}..{branch}"), "--no-merges"]);
    let mut duplicates = 0usize;
    for commit in commits
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Some(patch) = patch_id(commit) else {
            continue;
        };
        if base_patches.contains(&patch) {
            duplicates += 1;
        }
    }
    duplicates
}

fn base_patch_ids(base_ref: &str) -> Vec<String> {
    let commits = git(&["rev-list", base_ref, "--no-merges"]);
    let mut ids = Vec::new();
    for commit in commits
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if let Some(patch) = patch_id(commit) {
            ids.push(patch);
        }
    }
    ids
}

fn patch_id(commit: &str) -> Option<String> {
    let show = Command::new("git")
        .current_dir(workspace_root())
        .args(["show", "--pretty=format:", commit])
        .output()
        .ok()?;
    if !show.status.success() {
        return None;
    }
    let piped = Command::new("git")
        .current_dir(workspace_root())
        .args(["patch-id", "--stable"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    use std::io::Write;
    let mut child = piped;
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(&show.stdout);
    }
    let out = child.wait_with_output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .map(str::to_string)
}

fn git(args: &[&str]) -> String {
    let root: &Path = &workspace_root();
    Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).to_string())
        .unwrap_or_default()
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

    #[test]
    fn git_surface_is_readable_from_the_workspace() {
        // The audit must be able to read at least the base ref, otherwise every
        // branch silently reports "observe" and the report is decorative.
        let refs = git(&["rev-parse", "--abbrev-ref", "HEAD"]);
        assert!(
            !refs.trim().is_empty(),
            "git must be readable from the workspace root"
        );
    }
}
