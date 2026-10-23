//! Operator activity summary: pure rollup of per-forge repo counts.
//!
//! WHY: the operator's digital identity spans two forges (GitHub zeyxx,
//! Forgejo jeanterre13). This module turns per-repo open counts into one
//! sorted report. Pure logic, no I/O: the caller fetches
//! `ExternalRepoSummary` rows via `ExternalSourcePort::list_user_repos`
//! and hands them here.
//!
//! NEGATIVE BOUNDARY: no network, no files, no tokens. Counts only.

use cynic_vcs::ExternalRepoSummary;
use serde::{Deserialize, Serialize};

/// Schema identifier for collaborator activity reports.
pub const COLLABORATOR_ACTIVITY_SCHEMA: &str = "cynic.node.collaborator_activity.v1";

/// One forge's slice of the operator's activity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgeActivity {
    pub forge: String,
    pub user: String,
    pub repo_count: usize,
    pub open_issues: u64,
    pub open_prs: u64,
    pub repos: Vec<RepoActivity>,
}

/// One repo, hottest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoActivity {
    pub name: String,
    pub open_issues: u64,
    pub open_prs: Option<u64>,
    pub updated_at: String,
}

/// Whole report across forges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityReport {
    pub schema: String,
    pub forges: Vec<ForgeActivity>,
    pub total_repos: usize,
    pub total_open_issues: u64,
    pub total_open_prs: u64,
}

/// Roll up per-repo summaries into a sorted report.
///
/// - `forge` / `user` label the slice; `summaries` are that forge's repos.
/// - Repos sort by open issues descending, then by name.
/// - An optional `project` filter keeps a single repo (by name).
pub fn summarize_activity(
    forge: &str,
    user: &str,
    summaries: &[ExternalRepoSummary],
    project: Option<&str>,
) -> ForgeActivity {
    let mut repos: Vec<RepoActivity> = summaries
        .iter()
        .filter(|s| project.is_none_or(|wanted| s.name == wanted))
        .map(|s| RepoActivity {
            name: s.name.clone(),
            open_issues: s.open_issues,
            open_prs: s.open_prs,
            updated_at: s.updated_at.clone(),
        })
        .collect();
    repos.sort_by(|a, b| {
        b.open_issues
            .cmp(&a.open_issues)
            .then_with(|| a.name.cmp(&b.name))
    });
    let open_issues = repos.iter().map(|r| r.open_issues).sum();
    let open_prs = repos.iter().map(|r| r.open_prs.unwrap_or(0)).sum();
    ForgeActivity {
        forge: forge.to_string(),
        user: user.to_string(),
        repo_count: repos.len(),
        open_issues,
        open_prs,
        repos,
    }
}

/// Merge per-forge slices into the whole report.
pub fn merge_activity(forges: Vec<ForgeActivity>) -> ActivityReport {
    let total_repos = forges.iter().map(|f| f.repo_count).sum();
    let total_open_issues = forges.iter().map(|f| f.open_issues).sum();
    let total_open_prs = forges.iter().map(|f| f.open_prs).sum();
    ActivityReport {
        schema: COLLABORATOR_ACTIVITY_SCHEMA.to_string(),
        forges,
        total_repos,
        total_open_issues,
        total_open_prs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(owner: &str, name: &str, issues: u64, prs: Option<u64>) -> ExternalRepoSummary {
        ExternalRepoSummary {
            owner: owner.to_string(),
            name: name.to_string(),
            open_issues: issues,
            open_prs: prs,
            updated_at: "2026-10-23T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn hottest_repo_sorts_first() {
        let rows = vec![
            summary("zeyxx", "quiet", 0, None),
            summary("zeyxx", "busy", 7, None),
            summary("zeyxx", "mid", 3, None),
        ];
        let activity = summarize_activity("github", "zeyxx", &rows, None);
        assert_eq!(activity.repo_count, 3);
        assert_eq!(activity.open_issues, 10);
        assert_eq!(activity.repos[0].name, "busy");
        assert_eq!(activity.repos[2].name, "quiet");
    }

    #[test]
    fn project_filter_keeps_one_repo() {
        let rows = vec![
            summary("zeyxx", "alpha", 5, None),
            summary("zeyxx", "beta", 2, None),
        ];
        let activity = summarize_activity("github", "zeyxx", &rows, Some("beta"));
        assert_eq!(activity.repo_count, 1);
        assert_eq!(activity.repos[0].name, "beta");
        assert_eq!(activity.open_issues, 2);
    }

    #[test]
    fn empty_forge_reports_zero() {
        let activity = summarize_activity("forgejo", "jeanterre13", &[], None);
        assert_eq!(activity.repo_count, 0);
        assert_eq!(activity.open_issues, 0);
        assert_eq!(activity.open_prs, 0);
        assert!(activity.repos.is_empty());
    }

    #[test]
    fn merge_sums_forges() {
        let left = summarize_activity("github", "zeyxx", &[summary("zeyxx", "a", 4, None)], None);
        let right = summarize_activity(
            "forgejo",
            "jeanterre13",
            &[summary("jeanterre13", "b", 1, Some(2))],
            None,
        );
        let report = merge_activity(vec![left, right]);
        assert_eq!(report.total_repos, 2);
        assert_eq!(report.total_open_issues, 5);
        assert_eq!(report.total_open_prs, 2);
        assert_eq!(report.schema, COLLABORATOR_ACTIVITY_SCHEMA);
    }
}
