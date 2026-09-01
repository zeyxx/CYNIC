//! Pure types for the sovereign merge agent.
//!
//! No I/O here: these shapes are consumed by pure reconciliation
//! (`reconcile`) and produced by observation adapters (`observe`).
//! Substrate binding (git CLI, SSH, worktree) stays behind `port`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique merge request identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeRequestId(pub String);

impl MergeRequestId {
    pub fn new(source: &str, target: &str) -> Self {
        Self(format!("merge:{}→{}", source, target))
    }
}

/// Classification of a conflict marker found in the working tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictClass {
    /// Standard `<<<<<<< HEAD ... >>>>>>> branch` marker block.
    Content,
    /// Binary file conflict (no textual merge possible).
    Binary,
    /// File deleted in one branch, modified in the other.
    DeleteModify,
    /// File added in both branches with different content.
    AddAdd,
}

/// One detected conflict in a single file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    /// Path relative to the worktree root.
    pub path: String,
    pub class: ConflictClass,
    /// Number of conflicting hunks in this file.
    pub hunk_count: u32,
}

/// Full observation of a branch pair's merge state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeObservation {
    pub schema: String,
    pub source_branch: String,
    pub target_branch: String,
    /// Commits unique to source (not in target).
    pub commits_ahead: u32,
    /// Commits unique to target (not in source).
    pub commits_behind: u32,
    /// Files that differ between the two branch tips.
    pub files_changed: Vec<String>,
    /// Conflicts detected after a merge attempt (empty if clean).
    pub conflicts: Vec<Conflict>,
    /// Whether the merge is a fast-forward.
    pub is_fast_forward: bool,
}

/// Pure reconciliation decision from observation.
///
/// This type never authorizes mutation directly. It names the next safe
/// action and the residual uncertainty. `apply` consumes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeDecision {
    pub schema: String,
    pub request_id: MergeRequestId,
    /// Strategy selected by pure reconciliation.
    pub strategy: MergeStrategy,
    /// Confidence in this decision (0.0 = unknown, 1.0 = certain).
    pub confidence: f64,
    /// Whether human review is required before applying.
    pub requires_human_review: bool,
    /// Rationale — human-readable grounds for the decision.
    pub rationale: String,
}

/// Merge strategy selected by pure reconciliation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Simple fast-forward — no merge commit needed.
    FastForward,
    /// Automatic merge with auto-resolution (union, ours, theirs).
    AutoMerge {
        conflict_resolution: ConflictResolution,
    },
    /// Merge requires LLM-assisted resolution of conflicts.
    LlmAssist,
    /// Merge requires human review (uncertainty too high).
    HumanReview,
}

/// Conflict resolution strategy for auto-merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Keep both sides (union).
    Union,
    /// Prefer current branch (ours).
    Ours,
    /// Prefer incoming branch (theirs).
    Theirs,
    /// Per-file resolution map.
    PerFile(HashMap<String, ConflictResolution>),
}

/// Proof emitted after a successful merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeProof {
    pub schema: String,
    pub request_id: MergeRequestId,
    pub source_branch: String,
    pub target_branch: String,
    pub strategy: MergeStrategy,
    pub conflicts_resolved: u32,
    pub commit_sha: Option<String>,
    pub escalated: bool,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        ConflictResolution::Union
    }
}
