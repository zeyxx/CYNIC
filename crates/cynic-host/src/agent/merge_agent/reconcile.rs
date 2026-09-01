//! Pure reconciliation: select the next safe action from observation.
//!
//! This module owns the merge policy. No I/O, no mutation — only the
//! decision function. Consumed by `apply`.

use crate::agent::merge_agent::types::*;

/// Pure decision function from observation to merge strategy.
///
/// Rules (applied top-down, first match wins):
/// 1. Fast-forward → FastForward
/// 0 conflicts + ahead > 0 → AutoMerge (union)
/// 1-3 conflicts, all content, no binary/delete-modify → LlmAssist
/// >3 conflicts or any binary/delete-modify → HumanReview
pub fn reconcile(obs: &MergeObservation) -> MergeDecision {
    // Rule 1: Fast-forward
    if obs.is_fast_forward && obs.commits_behind == 0 && obs.conflicts.is_empty() {
        return MergeDecision {
            request_id: MergeRequestId::new(&obs.source_branch, &obs.target_branch),
            schema: "cynic.merge_decision.v1".to_string(),
            strategy: MergeStrategy::FastForward,
            confidence: 1.0,
            requires_human_review: false,
            rationale: format!(
                "Fast-forward: {} ahead, 0 behind, 0 conflicts",
                obs.commits_ahead
            ),
        };
    }

    // Rule 2: Clean auto-merge possible
    if obs.conflicts.is_empty() {
        return MergeDecision {
            request_id: MergeRequestId::new(&obs.source_branch, &obs.target_branch),
            schema: "cynic.merge_decision.v1".to_string(),
            strategy: MergeStrategy::AutoMerge {
                conflict_resolution: ConflictResolution::Union,
            },
            confidence: 0.95,
            requires_human_review: false,
            rationale: format!(
                "Clean merge: {} ahead, {} behind, 0 conflicts",
                obs.commits_ahead, obs.commits_behind
            ),
        };
    }

    // Rule 3: Check for hard conflicts
    let hard_conflicts = obs
        .conflicts
        .iter()
        .filter(|c| {
            matches!(
                c.class,
                ConflictClass::Binary | ConflictClass::DeleteModify | ConflictClass::AddAdd
            )
        })
        .count();

    // Rule 4: Too many conflicts or hard conflicts → human review
    if obs.conflicts.len() > 3 || hard_conflicts > 0 {
        return MergeDecision {
            request_id: MergeRequestId::new(&obs.source_branch, &obs.target_branch),
            schema: "cynic.merge_decision.v1".to_string(),
            strategy: MergeStrategy::HumanReview,
            confidence: 0.3,
            requires_human_review: true,
            rationale: format!(
                "Complex merge: {} conflicts ({} hard), human review required",
                obs.conflicts.len(),
                hard_conflicts
            ),
        };
    }

    // Rule 5: Small number of content conflicts → LLM assist
    MergeDecision {
        request_id: MergeRequestId::new(&obs.source_branch, &obs.target_branch),
        schema: "cynic.merge_decision.v1".to_string(),
        strategy: MergeStrategy::LlmAssist,
        confidence: 0.7,
        requires_human_review: false,
        rationale: format!(
            "LLM assist: {} content conflicts, auto-resolvable",
            obs.conflicts.len()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(conflicts: Vec<Conflict>, ahead: u32, behind: u32, ff: bool) -> MergeObservation {
        MergeObservation {
            schema: "cynic.merge_observation.v1".to_string(),
            source_branch: "test/source".to_string(),
            target_branch: "test/target".to_string(),
            commits_ahead: ahead,
            commits_behind: behind,
            files_changed: vec!["a.rs".to_string()],
            conflicts,
            is_fast_forward: ff,
        }
    }

    #[test]
    fn fast_forward_when_ahead_no_behind_no_conflicts() {
        let d = reconcile(&obs(vec![], 5, 0, true));
        assert!(matches!(d.strategy, MergeStrategy::FastForward));
        assert!(!d.requires_human_review);
    }

    #[test]
    fn auto_merge_when_clean_diverged() {
        let d = reconcile(&obs(vec![], 5, 3, false));
        assert!(matches!(
            d.strategy,
            MergeStrategy::AutoMerge {
                conflict_resolution: ConflictResolution::Union
            }
        ));
        assert!(!d.requires_human_review);
    }

    #[test]
    fn llm_assist_for_few_content_conflicts() {
        let d = reconcile(&obs(
            vec![
                Conflict {
                    path: "a.rs".to_string(),
                    class: ConflictClass::Content,
                    hunk_count: 1,
                },
                Conflict {
                    path: "b.rs".to_string(),
                    class: ConflictClass::Content,
                    hunk_count: 2,
                },
            ],
            5,
            3,
            false,
        ));
        assert!(matches!(d.strategy, MergeStrategy::LlmAssist));
    }

    #[test]
    fn human_review_for_many_conflicts() {
        let d = reconcile(&obs(
            vec![
                Conflict {
                    path: "a.rs".to_string(),
                    class: ConflictClass::Content,
                    hunk_count: 1,
                },
                Conflict {
                    path: "b.rs".to_string(),
                    class: ConflictClass::Content,
                    hunk_count: 1,
                },
                Conflict {
                    path: "c.rs".to_string(),
                    class: ConflictClass::Content,
                    hunk_count: 1,
                },
                Conflict {
                    path: "d.rs".to_string(),
                    class: ConflictClass::Content,
                    hunk_count: 1,
                },
            ],
            5,
            3,
            false,
        ));
        assert!(matches!(d.strategy, MergeStrategy::HumanReview));
    }

    #[test]
    fn human_review_for_binary_conflict() {
        let d = reconcile(&obs(
            vec![Conflict {
                path: "img.png".to_string(),
                class: ConflictClass::Binary,
                hunk_count: 1,
            }],
            5,
            3,
            false,
        ));
        assert!(matches!(d.strategy, MergeStrategy::HumanReview));
    }

    #[test]
    fn human_review_for_delete_modify() {
        let d = reconcile(&obs(
            vec![Conflict {
                path: "gone.rs".to_string(),
                class: ConflictClass::DeleteModify,
                hunk_count: 1,
            }],
            5,
            3,
            false,
        ));
        assert!(matches!(d.strategy, MergeStrategy::HumanReview));
    }
}
