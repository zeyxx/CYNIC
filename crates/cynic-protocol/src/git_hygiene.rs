//! Git hygiene — pure check for staged vs claim coherence.
//!
//! Complements `git_topology` (branch-level) with worktree-level hygiene:
//! are staged files within the claimed scope, is dirty on main, etc.
//! All checks are pure; observation is provided by the host adapter.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const GIT_HYGIENE_SCHEMA: &str = "cynic.git_hygiene.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HygieneViolation {
    /// Staged files present but no active claim
    StagedWithoutClaim,
    /// Staged files on main branch
    DirtyOnMain,
    /// Many staged files suggests `git add -A`
    BulkAddSuspected,
    /// Claim scope does not match staged paths (prefix mismatch)
    StagedOutsideClaim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHygieneObservation {
    pub schema: String,
    pub observed_at_unix_s: u64,
    pub current_branch: Option<String>,
    #[serde(default)]
    pub staged_files: Vec<String>,
    #[serde(default)]
    pub dirty_files: Vec<String>,
    /// Active claim branch if any (e.g. "agent/hermes/git-supervision")
    #[serde(default)]
    pub claim_branch: Option<String>,
    /// Claim scope path prefix if any (e.g. "crates/cynic-protocol")
    #[serde(default)]
    pub claim_scope: Option<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHygieneDecision {
    pub schema: String,
    pub observed_at_unix_s: u64,
    pub violations: Vec<HygieneViolation>,
    pub needs_review: bool,
    pub unknowns: Vec<String>,
}

pub fn check_hygiene(obs: &GitHygieneObservation) -> GitHygieneDecision {
    let mut violations = Vec::new();
    let unknowns = obs.unknowns.clone();

    // Dirty on main is always a violation
    if obs.current_branch.as_deref() == Some("main") && (!obs.staged_files.is_empty() || !obs.dirty_files.is_empty()) {
        violations.push(HygieneViolation::DirtyOnMain);
    }

    // Staged without claim
    if !obs.staged_files.is_empty() && obs.claim_branch.is_none() {
        violations.push(HygieneViolation::StagedWithoutClaim);
    }

    // Bulk add heuristic: > 10 staged files or contains many unrelated prefixes
    if obs.staged_files.len() > 10 {
        violations.push(HygieneViolation::BulkAddSuspected);
    }

    // Staged outside claim scope
    if let Some(scope) = &obs.claim_scope {
        if !scope.is_empty() {
            for f in &obs.staged_files {
                if !f.starts_with(scope.as_str()) && !f.starts_with(".agent") {
                    violations.push(HygieneViolation::StagedOutsideClaim);
                    break;
                }
            }
        }
    }

    // Deduplicate
    let mut uniq = HashSet::new();
    violations.retain(|v| uniq.insert(*v));

    let needs_review = !violations.is_empty() || !unknowns.is_empty();

    GitHygieneDecision {
        schema: GIT_HYGIENE_SCHEMA.to_string(),
        observed_at_unix_s: obs.observed_at_unix_s,
        violations,
        needs_review,
        unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(staged: Vec<&str>, branch: Option<&str>, claim: Option<&str>) -> GitHygieneObservation {
        GitHygieneObservation {
            schema: GIT_HYGIENE_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            current_branch: branch.map(|s| s.to_string()),
            staged_files: staged.into_iter().map(|s| s.to_string()).collect(),
            dirty_files: vec![],
            claim_branch: claim.map(|s| s.to_string()),
            claim_scope: claim.map(|s| s.to_string()),
            unknowns: vec![],
        }
    }

    #[test]
    fn staged_without_claim_is_violation() {
        let o = obs(vec!["crates/foo/mod.rs"], Some("agent/hermes/x"), None);
        let d = check_hygiene(&o);
        assert!(d.violations.contains(&HygieneViolation::StagedWithoutClaim));
    }

    #[test]
    fn staged_inside_claim_passes() {
        let o = GitHygieneObservation {
            schema: GIT_HYGIENE_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            current_branch: Some("agent/hermes/x".to_string()),
            staged_files: vec!["crates/cynic-protocol/src/foo.rs".to_string()],
            dirty_files: vec![],
            claim_branch: Some("agent/hermes/x".to_string()),
            claim_scope: Some("crates/cynic-protocol".to_string()),
            unknowns: vec![],
        };
        let d = check_hygiene(&o);
        assert!(!d.violations.contains(&HygieneViolation::StagedWithoutClaim));
        assert!(!d.violations.contains(&HygieneViolation::StagedOutsideClaim));
    }

    #[test]
    fn dirty_on_main() {
        let o = obs(vec!["foo.rs"], Some("main"), Some("main"));
        let d = check_hygiene(&o);
        assert!(d.violations.contains(&HygieneViolation::DirtyOnMain));
    }
}
