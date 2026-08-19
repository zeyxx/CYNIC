//! Source sync types — represent the 3-layer source topology.
//!
//! Cortex (worktree) → Forgejo (sovereign SSOT) → GitHub (public relay).
//!
//! These types are pure data. No I/O, no mutation. They carry the observed
//! state and the reconciliation decision.

use serde::{Deserialize, Serialize};

/// The three source layers in the sync topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLayer {
    /// Local worktree — where agents commit.
    Cortex,
    /// Sovereign SSOT — Forgejo.
    Forgejo,
    /// Public relay — GitHub.
    GitHub,
}

impl std::fmt::Display for SourceLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cortex => write!(f, "cortex"),
            Self::Forgejo => write!(f, "forgejo"),
            Self::GitHub => write!(f, "github"),
        }
    }
}

/// Observed state of a single source layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceState {
    pub layer: SourceLayer,
    /// Remote URL (SSH for forgejo, HTTPS for github).
    pub remote_url: Option<String>,
    /// Current HEAD commit SHA.
    pub head_sha: Option<String>,
    /// Current branch.
    pub branch: Option<String>,
    /// Number of commits ahead of the downstream layer.
    pub commits_ahead: Option<u32>,
    /// Number of commits behind the upstream layer.
    pub commits_behind: Option<u32>,
    /// Dirty files in the worktree (cortex only).
    pub dirty_files: Vec<String>,
    /// Whether the layer is reachable.
    pub reachable: bool,
    /// Blockers preventing sync.
    pub blockers: Vec<String>,
}

/// Observation of all three layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncObservation {
    pub schema: &'static str,
    pub cortex: SourceState,
    pub forgejo: SourceState,
    pub github: SourceState,
    pub timestamp_unix_s: u64,
}

/// A single sync action — bounded, explicit, reviewable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyncAction {
    /// Push cortex → forgejo.
    PushCortexToForgejo {
        commits: u32,
    },
    /// Push forgejo → github.
    PushForgejoToGithub {
        commits: u32,
    },
    /// Merge a branch into main on forgejo.
    MergeBranchIntoMain {
        branch: String,
        strategy: MergeStrategy,
    },
    /// Archive a branch before force-push.
    ArchiveBranch {
        branch: String,
    },
    /// No action needed — layers are converged.
    NoOp {
        reason: String,
    },
}

/// Merge strategy for branch consolidation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Standard merge commit.
    Merge,
    /// Squash all commits into one.
    Squash,
    /// Rebase branch onto main.
    Rebase,
}

/// The reconciliation decision — pure output from desired + observed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDecision {
    pub schema: &'static str,
    pub observation: SyncObservation,
    pub actions: Vec<SyncAction>,
    /// Whether the decision requires human review.
    pub needs_review: bool,
    /// Residual unknowns.
    pub unknowns: Vec<String>,
}

/// A sync plan — the bounded, explicit mutation to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPlan {
    pub schema: &'static str,
    pub decision: SyncDecision,
    /// Ordered list of commands to execute.
    pub steps: Vec<SyncStep>,
    /// Rollback intent on failure.
    pub rollback: RollbackIntent,
}

/// A single step in the sync plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyncStep {
    /// Execute a git command.
    GitCommand {
        argv: Vec<String>,
        layer: SourceLayer,
    },
    /// Write a file.
    WriteFile {
        path: String,
        content: String,
    },
    /// Archive a branch.
    Archive {
        branch: String,
        archive_ref: String,
    },
}

/// Rollback strategy on sync failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RollbackIntent {
    /// No rollback — manual intervention required.
    None,
    /// Reset cortex HEAD to pre-sync state.
    ResetCortex {
        to_sha: String,
    },
    /// Reset forgejo/main to pre-sync state.
    ResetForgejo {
        to_sha: String,
    },
}

/// Proof of sync execution — evidence after the action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProof {
    pub schema: &'static str,
    pub plan: SyncPlan,
    pub executed_at_unix_s: u64,
    pub steps_completed: Vec<SyncStepResult>,
    pub success: bool,
    pub residual_unknowns: Vec<String>,
}

/// Result of a single sync step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStepResult {
    pub step: SyncStep,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}
