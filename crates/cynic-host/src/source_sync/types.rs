//! Source sync types — represent the 3-layer source topology.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLayer {
    Cortex,
    Forgejo,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceState {
    pub layer: SourceLayer,
    pub remote_url: Option<String>,
    pub head_sha: Option<String>,
    pub branch: Option<String>,
    pub commits_ahead: Option<u32>,
    pub commits_behind: Option<u32>,
    pub dirty_files: Vec<String>,
    pub reachable: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncObservation {
    pub schema: &'static str,
    pub cortex: SourceState,
    pub forgejo: SourceState,
    pub github: SourceState,
    pub timestamp_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyncAction {
    PushCortexToForgejo { commits: u32 },
    PushForgejoToGithub { commits: u32 },
    MergeBranchIntoMain { branch: String, strategy: MergeStrategy },
    ArchiveBranch { branch: String },
    NoOp { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    Merge,
    Squash,
    Rebase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDecision {
    pub schema: &'static str,
    pub observation: SyncObservation,
    pub actions: Vec<SyncAction>,
    pub needs_review: bool,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPlan {
    pub schema: &'static str,
    pub decision: SyncDecision,
    pub steps: Vec<SyncStep>,
    pub rollback: RollbackIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyncStep {
    GitCommand { argv: Vec<String>, layer: SourceLayer },
    WriteFile { path: String, content: String },
    Archive { branch: String, archive_ref: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RollbackIntent {
    None,
    ResetCortex { to_sha: String },
    ResetForgejo { to_sha: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProof {
    pub schema: &'static str,
    pub plan: SyncPlan,
    pub executed_at_unix_s: u64,
    pub steps_completed: Vec<SyncStepResult>,
    pub success: bool,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStepResult {
    pub step: SyncStep,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}
