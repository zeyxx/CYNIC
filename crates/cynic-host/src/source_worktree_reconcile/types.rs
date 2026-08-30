//! Types and decisions for source worktree reconciliation and multi-agent Git convergence.
//!
//! Complies with the 8-path state machine and 4 deterministic guardrails defined in
//! `infra/SYSTEM_AGENTIC_MAPPING.md`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SOURCE_WORKTREE_RECONCILE_SCHEMA: &str = "cynic.host.source_worktree_reconcile_plan.v1";
pub const SOURCE_WORKTREE_APPLY_PROOF_SCHEMA: &str = "cynic.host.source_worktree_apply_proof.v1";

/// Comprehensive convergence plan produced by the pure reconciliation engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeConvergencePlan {
    pub schema: &'static str,
    pub repository_root: PathBuf,
    pub main_head_sha: String,
    pub observed_worktrees_count: usize,
    pub active_claims_count: usize,
    pub actions: Vec<WorktreeConvergenceAction>,
    pub summary: WorktreeConvergenceSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorktreeConvergenceSummary {
    pub fast_forward_ready_count: usize,
    pub rebase_required_count: usize,
    pub trivial_resolution_count: usize,
    pub quarantined_count: usize,
    pub pruned_zombies_count: usize,
    pub human_escalations_count: usize,
}

/// Discrete, bounded actions executable by the apply engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeConvergenceAction {
    /// Path 1: Instant linear fast-forward merge onto main.
    FastForwardMerge {
        branch: String,
        target_commit_sha: String,
        lease_id: Option<String>,
    },
    /// Path 2: Rebase branch onto current main head within scratch validator.
    RebaseInScratchValidator {
        branch: String,
        original_sha: String,
        onto_main_sha: String,
        workspace_path: PathBuf,
    },
    /// Path 3: Deterministic auto-resolution of manifest files (Cargo.lock, mod.rs).
    ResolveManifestConflict {
        branch: String,
        manifest_path: PathBuf,
    },
    /// Path 5: Quarantine branch due to detected semantic break or post-rebase test failure.
    QuarantineBranch {
        branch: String,
        reason: QuarantineReason,
        quarantine_ref: String,
    },
    /// Path 6: Prune zombie/abandoned worktree whose lease TTL expired.
    PruneZombieWorkspace {
        workspace_path: PathBuf,
        branch: Option<String>,
        lease_id: Option<String>,
        reason: String,
    },
    /// Path 8: Escalate unresolvable structural/architectural divergence to human sovereign HITL.
    EscalateToHuman {
        branch: String,
        conflicting_paths: Vec<String>,
        intent_summary: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuarantineReason {
    SilentSemanticBreak {
        broken_symbol: String,
        consumer_crate: String,
    },
    PostRebaseTestFailure {
        test_summary: String,
    },
    CausalBreach {
        details: String,
    },
}

/// Post-apply execution proof bundle with read-back evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeConvergenceApplyProof {
    pub schema: &'static str,
    pub applied_at_unix_s: u64,
    pub main_initial_sha: String,
    pub main_final_sha: String,
    pub executed_actions: Vec<ExecutedActionReceipt>,
    pub all_successful: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutedActionReceipt {
    pub action_name: &'static str,
    pub target_branch: Option<String>,
    pub success: bool,
    pub message: String,
    pub read_back_evidence: Option<String>,
}
