//! Git topology decision — pure policy, no I/O.
//!
//! Moved out of `cynic-protocol` (types and ports only): deciding what a
//! branch needs is policy, observing it is a port. The commit DAG stays
//! authoritative; review order is a derived stack forest (see `stack`);
//! mergeability is a verdict algebra (Commute / Depend / Conflict).

use std::collections::HashMap;

use cynic_protocol::git_topology::{GitTopologyObservation, GitTopologyUnknown};
use serde::{Deserialize, Serialize};

use super::verdict::{GitBranchVerdict, GitVerdict, verdict_for};

pub use cynic_protocol::git_topology::GitBranchState;

pub const GIT_RECONCILE_SCHEMA: &str = "cynic.git_reconcile.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GitAction {
    /// Branch is ready for PR (stable, ahead >0, behind small)
    ReadyForPr { branch: String },
    /// Needs rebase onto origin/main before PR
    NeedsRebase { branch: String, behind: u32 },
    /// Archive or delete — dead branch
    Archive { branch: String, reason: String },
    /// Split branch too large (many commits)
    Split { branch: String, ahead: u32 },
    /// Clean worktree or remove orphan worktree
    GcWorktree { branch: String },
    /// No action needed (on main, active, or already converged)
    NoOp { branch: String, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitReconcileDecision {
    pub schema: String,
    pub observed_at_unix_s: u64,
    pub actions: Vec<GitAction>,
    pub verdicts: Vec<GitBranchVerdict>,
    pub needs_review: bool,
    pub unknowns: Vec<String>,
}

/// Pure reconcile: topology -> actions + verdicts. No I/O, no git calls.
pub fn reconcile_git_topology(obs: &GitTopologyObservation) -> GitReconcileDecision {
    let mut actions: Vec<GitAction> = Vec::new();
    let mut unknowns = Vec::new();

    if !obs.observation_unknowns.is_empty() {
        unknowns.push(format!("observation_unknowns: {:?}", obs.observation_unknowns));
    }

    // Stack forest: branch -> parent, for depth. Unknown parents stay unknown.
    let parent_map: HashMap<String, String> = obs
        .entries
        .iter()
        .filter_map(|e| e.parent.clone().map(|p| (e.branch.clone(), p)))
        .collect();

    for e in &obs.entries {
        // Skip main itself
        if e.branch == "main" {
            actions.push(GitAction::NoOp {
                branch: e.branch.clone(),
                reason: "main".to_string(),
            });
            continue;
        }
        // Unknown-heavy entries need review, not action.
        // Dirty unknown for non-current is expected (we only observe dirty for current) — don't block.
        // Parent unknown for main-adjacent inference is tolerated: depth stays None, action proceeds.
        let is_current = obs.current_branch.as_deref() == Some(e.branch.as_str());
        let filtered_unknowns: Vec<_> = e
            .unknowns
            .iter()
            .filter(|u| {
                // Ignore Dirty for non-current branches
                !(!is_current && **u == GitTopologyUnknown::Dirty)
                // Ignore Parent: stack depth degrades to None, verdict still computable
                && **u != GitTopologyUnknown::Parent
            })
            .copied()
            .collect();
        if !filtered_unknowns.is_empty() {
            unknowns.push(format!("{}: unknowns {:?}", e.branch, filtered_unknowns));
            continue;
        }
        let behind = e.behind.unwrap_or(0);
        let ahead = e.ahead.unwrap_or(0);

        if is_current && e.dirty_count.unwrap_or(0) > 0 {
            let dirty_count = e.dirty_count.unwrap_or(0);
            actions.push(GitAction::NoOp {
                branch: e.branch.clone(),
                reason: format!("current worktree has {dirty_count} uncommitted paths"),
            });
            unknowns.push(format!(
                "{}: current worktree has {dirty_count} uncommitted paths",
                e.branch
            ));
            continue;
        }

        match e.state {
            GitBranchState::Dead => {
                // Data-centric: dead only if nothing to save (ahead==0). If ahead>0, treat as split/rebase — preserve value.
                if ahead > 0 && ahead >= 5 {
                    actions.push(GitAction::Split {
                        branch: e.branch.clone(),
                        ahead,
                    });
                } else if ahead > 0 {
                    actions.push(GitAction::NeedsRebase {
                        branch: e.branch.clone(),
                        behind,
                    });
                } else {
                    actions.push(GitAction::Archive {
                        branch: e.branch.clone(),
                        reason: format!("dead: {behind} behind main"),
                    });
                }
            }
            GitBranchState::Divergent => {
                actions.push(GitAction::NeedsRebase {
                    branch: e.branch.clone(),
                    behind,
                });
            }
            GitBranchState::Active | GitBranchState::Stable => {
                if ahead >= 20 {
                    actions.push(GitAction::Split {
                        branch: e.branch.clone(),
                        ahead,
                    });
                } else if ahead > 0 && behind <= 5 {
                    actions.push(GitAction::ReadyForPr {
                        branch: e.branch.clone(),
                    });
                } else if behind > 5 {
                    actions.push(GitAction::NeedsRebase {
                        branch: e.branch.clone(),
                        behind,
                    });
                } else {
                    actions.push(GitAction::NoOp {
                        branch: e.branch.clone(),
                        reason: "active, nothing to do".to_string(),
                    });
                }
            }
            GitBranchState::Orphan => {
                // Local-only branch, no remote — needs push or archive
                if ahead == 0 && behind == 0 {
                    unknowns.push(format!("orphan without remote: {}", e.branch));
                } else {
                    actions.push(GitAction::NoOp {
                        branch: e.branch.clone(),
                        reason: "orphan".to_string(),
                    });
                }
            }
        }
        // Worktree GC hint: divergent/dead with worktree
        if e.has_worktree == Some(true)
            && matches!(e.state, GitBranchState::Dead | GitBranchState::Divergent)
        {
            actions.push(GitAction::GcWorktree {
                branch: e.branch.clone(),
            });
        }
    }

    let needs_review = !unknowns.is_empty()
        || actions.iter().any(|a| {
            matches!(
                a,
                GitAction::Archive { .. } | GitAction::Split { .. }
            )
        });

    let max_depth = obs.entries.len();
    let verdicts = actions
        .iter()
        .map(|action| {
            let branch = match action {
                GitAction::ReadyForPr { branch }
                | GitAction::NeedsRebase { branch, .. }
                | GitAction::Archive { branch, .. }
                | GitAction::Split { branch, .. }
                | GitAction::GcWorktree { branch }
                | GitAction::NoOp { branch, .. } => branch.clone(),
            };
            let stack_depth = if branch == "main" {
                Some(0)
            } else {
                crate::domain::git::stack::stack_depth(&branch, &parent_map, max_depth)
            };
            GitBranchVerdict {
                branch,
                verdict: verdict_for(action),
                action: action.clone(),
                stack_depth,
            }
        })
        .collect();

    GitReconcileDecision {
        schema: GIT_RECONCILE_SCHEMA.to_string(),
        observed_at_unix_s: obs.observed_at_unix_s,
        actions,
        verdicts,
        needs_review,
        unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::git_topology::{
        GitBranchEntry, GitTopologyObservation, GIT_TOPOLOGY_SCHEMA,
    };

    fn valid_entry(branch: &str) -> GitBranchEntry {
        GitBranchEntry {
            branch: branch.to_string(),
            ahead: Some(2),
            behind: Some(0),
            last_commit_unix_s: Some(1_700_000_000),
            state: GitBranchState::Active,
            has_worktree: Some(false),
            dirty_count: Some(0),
            has_remote: Some(true),
            parent: Some("main".to_string()),
            unknowns: vec![],
        }
    }

    fn obs_with(entries: Vec<GitBranchEntry>, current: Option<&str>) -> GitTopologyObservation {
        GitTopologyObservation {
            schema: GIT_TOPOLOGY_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            node_id: "n".to_string(),
            current_branch: current.map(str::to_string),
            entries,
            observation_unknowns: vec![],
        }
    }

    #[test]
    fn dirty_current_branch_requires_review_not_pr() {
        let mut entry = valid_entry("agent/hermes/dirty");
        entry.dirty_count = Some(2);
        let d = reconcile_git_topology(&obs_with(vec![entry], Some("agent/hermes/dirty")));
        assert!(d.needs_review);
        assert!(d.actions.iter().any(|action| {
            matches!(action, GitAction::NoOp { branch, reason }
                if branch == "agent/hermes/dirty" && reason.contains("2 uncommitted paths"))
        }));
        assert!(!d
            .actions
            .iter()
            .any(|action| matches!(action, GitAction::ReadyForPr { .. })));
        assert!(d.verdicts.iter().any(|v| matches!(
            v,
            GitBranchVerdict { verdict: GitVerdict::Conflict { .. }, .. }
            if v.branch == "agent/hermes/dirty"
        )));
    }

    #[test]
    fn divergent_needs_rebase_and_depend_verdict() {
        let mut e = valid_entry("agent/hermes/div");
        e.behind = Some(60);
        e.state = GitBranchState::Divergent;
        let main = GitBranchEntry {
            branch: "main".to_string(),
            ahead: Some(0),
            behind: Some(0),
            last_commit_unix_s: Some(1),
            state: GitBranchState::Stable,
            has_worktree: Some(false),
            dirty_count: Some(0),
            has_remote: Some(true),
            parent: None,
            unknowns: vec![GitTopologyUnknown::Parent],
        };
        let d = reconcile_git_topology(&obs_with(vec![main, e], Some("main")));
        assert!(d.actions.iter().any(
            |a| matches!(a, GitAction::NeedsRebase { branch, .. } if branch == "agent/hermes/div")
        ));
        assert!(d.verdicts.iter().any(
            |v| matches!(v, GitBranchVerdict { verdict: GitVerdict::Depend { behind: 60 }, .. })
        ));
    }

    #[test]
    fn dead_archives_with_conflict_verdict() {
        let mut e = valid_entry("agent/hermes/dead");
        e.ahead = Some(0);
        e.behind = Some(150);
        e.state = GitBranchState::Dead;
        let d = reconcile_git_topology(&obs_with(vec![e], None));
        assert!(d.needs_review);
        assert!(d
            .actions
            .iter()
            .any(|a| matches!(a, GitAction::Archive { .. })));
        assert!(d
            .verdicts
            .iter()
            .any(|v| matches!(v.verdict, GitVerdict::Conflict { .. })));
    }

    #[test]
    fn dead_with_ahead_becomes_split_not_archive() {
        let mut e = valid_entry("agent/hermes/dead-ahead");
        e.ahead = Some(42);
        e.behind = Some(134);
        e.state = GitBranchState::Dead;
        let d = reconcile_git_topology(&obs_with(vec![e], None));
        assert!(d.actions.iter().any(
            |a| matches!(a, GitAction::Split { branch, .. } if branch == "agent/hermes/dead-ahead")
        ));
        assert!(!d
            .actions
            .iter()
            .any(|a| matches!(a, GitAction::Archive { .. })));
    }

    #[test]
    fn ready_for_pr_commutes_with_stack_depth() {
        let e = valid_entry("agent/hermes/ship");
        let d = reconcile_git_topology(&obs_with(vec![e], Some("main")));
        let v = d
            .verdicts
            .iter()
            .find(|v| v.branch == "agent/hermes/ship")
            .expect("verdict");
        assert_eq!(v.verdict, GitVerdict::Commute);
        assert_eq!(v.stack_depth, Some(1));
    }

    #[test]
    fn missing_parent_keeps_action_but_no_depth() {
        let mut e = valid_entry("agent/hermes/noparent");
        e.parent = None;
        e.unknowns = vec![GitTopologyUnknown::Parent];
        let d = reconcile_git_topology(&obs_with(vec![e], Some("main")));
        // Parent unknown tolerated: action proceeds (ReadyForPr), depth unknown.
        assert!(d
            .actions
            .iter()
            .any(|a| matches!(a, GitAction::ReadyForPr { .. })));
        let v = d
            .verdicts
            .iter()
            .find(|v| v.branch == "agent/hermes/noparent")
            .expect("verdict");
        assert_eq!(v.stack_depth, None);
    }
}
