//! Mergeability verdicts — pijul-style algebra over git actions.
//!
//! A verdict answers one question: can this branch move toward main now,
//! and if not, what stands in the way? Documented heuristic, not proof:
//! Commute (safe to propose), Depend (prior action first), Conflict (human).

use serde::{Deserialize, Serialize};

use super::decision::GitAction;

/// Mergeability verdict (documented heuristic).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GitVerdict {
    /// Applies onto main in any order — safe to propose.
    Commute,
    /// Needs a prior action first (rebase onto main).
    Depend { behind: u32 },
    /// Cannot auto-proceed — human review required.
    Conflict { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitBranchVerdict {
    pub branch: String,
    pub verdict: GitVerdict,
    pub action: GitAction,
    /// Depth in the review stack (main itself = 0).
    /// None when the parent chain is unknown, cyclic, or over budget.
    pub stack_depth: Option<u32>,
}

pub fn verdict_for(action: &GitAction) -> GitVerdict {
    match action {
        GitAction::ReadyForPr { .. } => GitVerdict::Commute,
        GitAction::NeedsRebase { behind, .. } => GitVerdict::Depend { behind: *behind },
        GitAction::Archive { reason, .. } => GitVerdict::Conflict {
            reason: format!("archive: {reason}"),
        },
        GitAction::Split { ahead, .. } => GitVerdict::Conflict {
            reason: format!("split: {ahead} ahead"),
        },
        GitAction::GcWorktree { .. } => GitVerdict::Conflict {
            reason: "worktree gc".to_string(),
        },
        GitAction::NoOp { reason, .. } => {
            if reason.contains("uncommitted paths") {
                GitVerdict::Conflict {
                    reason: reason.clone(),
                }
            } else {
                GitVerdict::Commute
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_commutes_rebase_depends() {
        assert_eq!(
            verdict_for(&GitAction::ReadyForPr {
                branch: "b".to_string()
            }),
            GitVerdict::Commute
        );
        assert_eq!(
            verdict_for(&GitAction::NeedsRebase {
                branch: "b".to_string(),
                behind: 7
            }),
            GitVerdict::Depend { behind: 7 }
        );
    }

    #[test]
    fn destructive_actions_conflict() {
        assert!(matches!(
            verdict_for(&GitAction::Archive {
                branch: "b".to_string(),
                reason: "dead".to_string()
            }),
            GitVerdict::Conflict { .. }
        ));
        assert!(matches!(
            verdict_for(&GitAction::Split {
                branch: "b".to_string(),
                ahead: 42
            }),
            GitVerdict::Conflict { .. }
        ));
    }

    #[test]
    fn dirty_noop_conflicts_clean_noop_commutes() {
        assert!(matches!(
            verdict_for(&GitAction::NoOp {
                branch: "b".to_string(),
                reason: "current worktree has 2 uncommitted paths".to_string()
            }),
            GitVerdict::Conflict { .. }
        ));
        assert_eq!(
            verdict_for(&GitAction::NoOp {
                branch: "b".to_string(),
                reason: "active, nothing to do".to_string()
            }),
            GitVerdict::Commute
        );
    }
}
