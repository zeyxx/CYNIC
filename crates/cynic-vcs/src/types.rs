//! Pure domain types for the sovereign VCS domain.
//!
//! Consolidated from the former `sync/types`, `source_worktree`,
//! `source_worktree_inventory`, `source_control`, `source_materialization`,
//! `source_release`, `source_repository_bootstrap`, `forgejo` and
//! `forgejo_token` type definitions. Zero I/O: data, schema constants and
//! decision inputs only.

use cynic_network::NetworkPathEvidence;
use cynic_protocol::LeaseGuard;
use cynic_protocol::SecurityEvidenceState;
use cynic_protocol::SecurityEvidenceState as ForgejoEvidenceState;
use cynic_protocol::vcs::source_worktree_lease_target;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Admission verdict for a source sync plan.
///
/// Mirrors the `PolicyAdmissionReport` pattern in `cynic-protocol` but
/// specialized for the source-control domain. A sync plan is only
/// executable when `admitted == true` and `blockers.is_empty()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceAdmissionReport {
    pub schema: &'static str,
    pub decision_digest: String,
    pub gates_evaluated: Vec<String>,
    pub gates_passed: Vec<String>,
    pub gates_failed: Vec<String>,
    pub admitted: bool,
    pub blockers: Vec<String>,
}

impl SourceAdmissionReport {
    /// Construct a blocked admission with the given gate failures.
    pub fn blocked(
        decision: &SyncDecision,
        gates_passed: Vec<String>,
        gates_failed: Vec<String>,
        blockers: Vec<String>,
    ) -> Self {
        Self {
            schema: SOURCE_ADMISSION_SCHEMA,
            decision_digest: format!("{:p}", decision),
            gates_evaluated: gates_passed
                .iter()
                .chain(gates_failed.iter())
                .cloned()
                .collect(),
            gates_passed,
            gates_failed,
            admitted: false,
            blockers,
        }
    }

    /// Construct an admitted report.
    pub fn admitted(
        decision: &SyncDecision,
        gates_evaluated: Vec<String>,
        gates_passed: Vec<String>,
    ) -> Self {
        Self {
            schema: SOURCE_ADMISSION_SCHEMA,
            decision_digest: format!("{:p}", decision),
            gates_evaluated,
            gates_passed,
            gates_failed: Vec::new(),
            admitted: true,
            blockers: Vec::new(),
        }
    }
}

pub const SOURCE_ADMISSION_SCHEMA: &str = "cynic.source_admission.v1";

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
    pub remote_name: Option<String>,
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
    PushCortexToForgejo {
        commits: u32,
    },
    PushForgejoToGithub {
        commits: u32,
    },
    MergeBranchIntoMain {
        branch: String,
        strategy: MergeStrategy,
    },
    ArchiveBranch {
        branch: String,
    },
    NoOp {
        reason: String,
    },
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
    GitCommand {
        argv: Vec<String>,
        layer: SourceLayer,
    },
    WriteFile {
        path: String,
        content: String,
    },
    Archive {
        branch: String,
        archive_ref: String,
    },
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

pub const SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA: &str =
    "cynic.source_worktree.convergence_plan.v1";
pub const SOURCE_WORKTREE_APPLY_PROOF_SCHEMA: &str = "cynic.source_worktree.apply_proof.v1";

/// Snapshot of an observed worktree passed into the pure decision function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedWorktreeSnapshot {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub head_sha: String,
    pub is_active_root: bool,
    pub dirty_file_count: usize,
    pub git_prunable: bool,
    pub prunable_reason: Option<String>,
}

/// Discrete, bounded actions decidable by the pure reconciliation engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum WorktreeConvergenceAction {
    /// 1. Branch is clean, fully merged into main HEAD -> safe to remove worktree and delete local branch
    PruneMergedWorkspace {
        workspace_path: PathBuf,
        branch: String,
        commit_sha: String,
        reason: String,
    },
    /// 2. Branch is ahead of main with no conflicts -> eligible for integration
    EligibleForIntegration {
        workspace_path: PathBuf,
        branch: String,
        target_commit_sha: String,
        ahead_count: usize,
    },
    /// 3. Branch diverged from main -> requires explicit human review
    EscalateDivergence {
        workspace_path: PathBuf,
        branch: String,
        branch_head: String,
        onto_main_sha: String,
        reason: String,
    },
    /// 4. Worktree is marked prunable or detached without valid git dir -> safe to force remove
    PruneZombieWorkspace {
        workspace_path: PathBuf,
        branch: Option<String>,
        reason: String,
    },
    /// 5. Active session, uncommitted changes, or active coordination lease -> protected
    RetainActiveWorkspace {
        workspace_path: PathBuf,
        branch: Option<String>,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorktreeConvergenceSummary {
    pub total_observed: usize,
    pub safe_prune_ready_count: usize,
    pub integration_eligible_count: usize,
    pub retained_active_count: usize,
    pub escalation_required_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeConvergencePlan {
    pub schema: &'static str,
    pub repository_root: PathBuf,
    pub default_branch: String,
    pub main_head_sha: String,
    pub actions: Vec<WorktreeConvergenceAction>,
    pub summary: WorktreeConvergenceSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutedActionReceipt {
    pub action_name: &'static str,
    pub target_path: PathBuf,
    pub target_branch: Option<String>,
    pub success: bool,
    pub message: String,
    pub read_back_evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeConvergenceApplyProof {
    pub schema: &'static str,
    pub applied_at_unix_s: u64,
    pub repository_root: PathBuf,
    pub main_initial_sha: String,
    pub main_final_sha: String,
    pub executed_actions: Vec<ExecutedActionReceipt>,
    pub all_successful: bool,
}

pub const SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA: &str =
    "cynic.source_worktree_inventory_observation.v2";
pub const SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES: usize = 4096;
pub const SOURCE_WORKTREE_INVENTORY_MAX_STRING_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWorktreeInventoryUnknown {
    Cleanliness,
    DetachedHead,
    GitHead,
    GitPrunable,
    GitRegistration,
    PathSymlink,
    RelativePath,
    Principal,
    Branch,
    Bare,
    Locked,
    LockReason,
    PrunableReason,
    GitAttribute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWorktreeInventoryPathBasis {
    MainRepository,
    WorkspaceLayout,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWorktreeInventoryEntryState {
    Observed,
    OutsideDeclaredLayout,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWorktreeInventoryObservationUnknown {
    GitCommandFailed,
    EmptyOutput,
    OutputLimitExceeded,
    EntryLimitExceeded,
    LayoutUnavailable,
    MalformedRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceWorktreeInventoryEntry {
    /// Opaque identity assigned by the adapter to one observed Git worktree.
    pub candidate_id: String,
    /// Canonical digest of the adapter evidence, pending independent verification.
    pub evidence_digest: String,
    pub state: SourceWorktreeInventoryEntryState,
    pub unknown_attribute_count: u16,
    pub path_basis: SourceWorktreeInventoryPathBasis,
    /// Relative to the declared `path_basis`; never an absolute machine path.
    #[serde(default)]
    pub relative_path: Option<String>,
    #[serde(default)]
    pub head: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub observed_principal: Option<String>,
    #[serde(default)]
    pub path_is_symlink: Option<bool>,
    #[serde(default)]
    pub git_registered: Option<bool>,
    #[serde(default)]
    pub git_prunable: Option<bool>,
    #[serde(default)]
    pub detached_head: Option<bool>,
    #[serde(default)]
    pub clean: Option<bool>,
    #[serde(default)]
    pub locked: Option<bool>,
    #[serde(default)]
    pub bare: Option<bool>,
    #[serde(default)]
    pub lock_reason: Option<String>,
    #[serde(default)]
    pub prunable_reason: Option<String>,
    #[serde(default)]
    pub unknowns: Vec<SourceWorktreeInventoryUnknown>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceWorktreeInventoryObservation {
    pub schema: String,
    pub repository_id: String,
    pub node_id: String,
    pub observed_at_ms: u64,
    #[serde(default)]
    pub entries: Vec<SourceWorktreeInventoryEntry>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub observation_unknowns: Vec<SourceWorktreeInventoryObservationUnknown>,
}

pub const SOURCE_ADMISSION_REPORT_SCHEMA: &str = "cynic.source.admission.v1";
pub const SOURCE_CHANGE_PROPOSAL_SCHEMA: &str = "cynic.source.change_proposal.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceMatterClass {
    Genome,
    DesiredState,
    Adapter,
    Evidence,
    GeneratedArtifact,
    Secret,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceChangeProposal {
    pub schema: String,
    pub proposal_id: String,
    pub repository_id: String,
    pub worktree_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub base_commit: String,
    pub actor: String,
    pub node_id: String,
    pub intent: String,
    pub matter: SourceMatterClass,
    pub paths: Vec<String>,
    pub validation: SecurityEvidenceState,
    pub rollback: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl SourceChangeProposal {
    pub fn lease_guard(&self) -> LeaseGuard {
        LeaseGuard {
            agent_id: self.actor.clone(),
            target: source_worktree_lease_target(&self.repository_id, &self.worktree_id),
            lease_id: self.lease_id.clone(),
            fencing_token: self.fencing_token,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceChangeAdmission {
    pub schema: String,
    pub status: String,
    pub proposal_id: String,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePromotionStage {
    Worktree,
    CommitCandidate,
    SovereignCommit,
    ReleaseSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceStageAdmissionReport {
    pub schema: String,
    pub status: String,
    pub target_stage: SourcePromotionStage,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub const SOURCE_MATERIALIZATION_REQUEST_SCHEMA: &str = "cynic.source.materialization_request.v1";
pub const SOURCE_MATERIALIZATION_OBSERVATION_SCHEMA: &str =
    "cynic.source.materialization_observation.v1";
pub const SOURCE_MATERIALIZATION_PLAN_SCHEMA: &str = "cynic.source.materialization_plan.v1";
pub const SOURCE_MATERIALIZATION_FORMAT: &str = "git_bundle_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMaterializationRequest {
    pub schema: String,
    pub release_id: String,
    pub repository_id: String,
    pub source_commit: String,
    #[serde(default)]
    pub source_tree: Option<String>,
    pub snapshot_digest: String,
    pub snapshot_format: String,
    pub target_node_id: String,
    pub target_relative_path: String,
    pub required_principal: String,
    pub retention_until_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMaterializationObservation {
    pub schema: String,
    pub observed_at_unix_ms: u64,
    pub target_node_id: String,
    pub target_role: Option<String>,
    pub node_identity: SecurityEvidenceState,
    pub running_as_required_principal: SecurityEvidenceState,
    pub snapshot_present: bool,
    pub snapshot_digest_read_back: Option<String>,
    pub snapshot_format_read_back: Option<String>,
    pub target_exists: bool,
    pub target_is_isolated: SecurityEvidenceState,
    pub observed_commit: Option<String>,
    #[serde(default)]
    pub observed_tree: Option<String>,
    pub observed_snapshot_digest: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SourceMaterializationAction {
    AcquireSnapshot {
        snapshot_digest: String,
        snapshot_format: String,
    },
    MaterializeCheckout {
        snapshot_digest: String,
        source_commit: String,
        target_relative_path: String,
    },
    ReadBack {
        snapshot_digest: String,
        source_commit: String,
        target_relative_path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMaterializationPlan {
    pub schema: String,
    pub status: String,
    pub release_id: String,
    pub action_budget: u32,
    pub actions: Vec<SourceMaterializationAction>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
    pub stop_conditions: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub const SOURCE_RELEASE_REQUEST_SCHEMA: &str = "cynic.source.release_request.v1";
pub const SOURCE_RELEASE_OBSERVATION_SCHEMA: &str = "cynic.source.release_observation.v1";
pub const SOURCE_RELEASE_PLAN_SCHEMA: &str = "cynic.source.release_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReleaseRequest {
    pub schema: String,
    pub release_id: String,
    pub proposal_id: String,
    pub repository_id: String,
    pub sovereign_remote: String,
    pub sovereign_ref: String,
    pub source_commit: String,
    pub build_runner_node_id: String,
    pub retention_until_unix_ms: u64,
    pub requested_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReleaseObservation {
    pub schema: String,
    pub observed_at_unix_ms: u64,
    pub proposal_admission: SecurityEvidenceState,
    pub worktree_clean: SecurityEvidenceState,
    pub sovereign_transport_identity: SecurityEvidenceState,
    pub sovereign_authentication: SecurityEvidenceState,
    pub remote_ref_commit: Option<String>,
    pub snapshot_digest: Option<String>,
    pub snapshot_digest_algorithm: Option<String>,
    pub snapshot_store_read_back: SecurityEvidenceState,
    pub build_runner_role: Option<String>,
    pub build_runner_identity: SecurityEvidenceState,
    pub build_runner_materialization_port: SecurityEvidenceState,
    pub grant_signing_authority: SecurityEvidenceState,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReleasePlan {
    pub schema: String,
    pub status: String,
    pub release_id: String,
    pub source_commit: String,
    pub snapshot_digest: Option<String>,
    pub build_runner_node_id: String,
    pub action_budget: u32,
    pub actions: Vec<SourceReleaseAction>,
    pub stop_conditions: Vec<String>,
    pub retention_until_unix_ms: u64,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SourceReleaseAction {
    MaterializeSnapshot {
        repository_id: String,
        source_commit: String,
        snapshot_digest: String,
        target_node_id: String,
    },
    VerifyCheckout {
        source_commit: String,
        snapshot_digest: String,
        target_node_id: String,
    },
    DispatchBuildWorkload {
        source_commit: String,
        target_node_id: String,
        requested_action: String,
        retention_until_unix_ms: u64,
    },
}

pub const SOURCE_REPOSITORY_BOOTSTRAP_REQUEST_SCHEMA: &str =
    "cynic.source_repository_bootstrap_request.v1";
pub const SOURCE_REPOSITORY_BOOTSTRAP_OBSERVATION_SCHEMA: &str =
    "cynic.source_repository_bootstrap_observation.v1";
pub const SOURCE_REPOSITORY_BOOTSTRAP_REPORT_SCHEMA: &str =
    "cynic.source_repository_bootstrap_report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRepositoryBootstrapRequest {
    pub schema: String,
    pub node_id: String,
    pub repository_id: String,
    pub source: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRepositoryBootstrapObservation {
    pub schema: String,
    pub node_id: String,
    pub repository_id: String,
    pub repository_path: String,
    pub required_principal: String,
    pub state_root_exists: bool,
    pub state_root_is_directory: bool,
    pub state_root_owned_by_principal: bool,
    pub running_as_required_principal: bool,
    pub repository_exists: bool,
    pub repository_is_git: bool,
    pub observed_revision: Option<String>,
    pub observed_source: Option<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum SourceRepositoryBootstrapDecision {
    Converged,
    BootstrapRequired,
    SyncRequired,
    Blocked { blockers: Vec<String> },
    Unknown { unknowns: Vec<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRepositoryBootstrapStatus {
    Planned,
    Completed,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRepositoryBootstrapProofAuthority {
    #[default]
    Uncommitted,
    LocalContentAddressed,
    KernelCommitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRepositoryBootstrapReport {
    pub schema: String,
    pub status: SourceRepositoryBootstrapStatus,
    pub request: SourceRepositoryBootstrapRequest,
    pub before: SourceRepositoryBootstrapObservation,
    pub decision: SourceRepositoryBootstrapDecision,
    #[serde(default)]
    pub planned_actions: Vec<Vec<String>>,
    pub after: Option<SourceRepositoryBootstrapObservation>,
    pub lease: Option<cynic_protocol::LeaseGuard>,
    pub proof_ref: Option<String>,
    #[serde(default)]
    pub proof_authority: SourceRepositoryBootstrapProofAuthority,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

pub const FORGEJO_PLAN_SCHEMA: &str = "cynic.forgejo.plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoDesiredState {
    pub instance_id: String,
    pub owner: String,
    pub repository: String,
    pub api_url: String,
    pub git_ssh_url: String,
    pub machine_principal: String,
    pub repository_permission: ForgejoRepositoryPermission,
    pub machine_key_fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgejoRepositoryPermission {
    Read,
    Write,
    Admin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoObservedState {
    pub admin_path: NetworkPathEvidence,
    pub owner_active: ForgejoEvidenceState,
    pub repository_exists: ForgejoEvidenceState,
    pub machine_principal_ready: ForgejoEvidenceState,
    pub machine_key_enrolled: ForgejoEvidenceState,
    pub repository_permission_granted: ForgejoEvidenceState,
    pub ssh_host_identity: ForgejoEvidenceState,
    pub git_ssh_authenticated: ForgejoEvidenceState,
    pub bootstrap_authority_revoked: ForgejoEvidenceState,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgejoActionKind {
    ObserveApiIdentity,
    ActivateOwner,
    CreateRepository,
    EnsureMachinePrincipal,
    EnrollMachineKey,
    GrantRepositoryPermission,
    VerifySshHostIdentity,
    VerifyGitSshAuthentication,
    RevokeBootstrapAuthority,
    /// Provision (create/rotate) the Forgejo API token for the machine principal.
    /// Does NOT require a human admin secret — runs through the SSH-local
    /// execution surface where `cynic-cortex` is enrolled.
    ProvisionForgejoToken,
    /// Observe (probe) the Forgejo API token file metadata + API validity.
    ObserveForgejoToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoPlannedAction {
    pub kind: ForgejoActionKind,
    pub mutates: bool,
    pub requires_human_admin_secret: bool,
    pub proof: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoPlan {
    pub schema: String,
    pub status: String,
    pub desired: ForgejoDesiredState,
    pub actions: Vec<ForgejoPlannedAction>,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub const FORGEJO_TOKEN_OBSERVATION_SCHEMA: &str = "cynic.forgejo_token.observation.v1";
pub const FORGEJO_TOKEN_SPEC_SCHEMA: &str = "cynic.forgejo_token.spec.v1";
pub const FORGEJO_TOKEN_STATUS_SCHEMA: &str = "cynic.forgejo_token.status.v1";
pub const FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA: &str = "cynic.forgejo_token.provision_report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForgejoTokenAuthState {
    Unknown,
    Proven,
    Refuted,
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForgejoTokenActionKind {
    Observe,
    Create,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenSpec {
    pub schema: String,
    pub principal: String,
    pub api_url: String,
    pub target_path: String,
    pub mode: u32,
    pub scopes: Vec<String>,
    pub expiry_seconds: Option<u64>,
}

impl Default for ForgejoTokenSpec {
    fn default() -> Self {
        Self {
            schema: FORGEJO_TOKEN_SPEC_SCHEMA.to_string(),
            principal: String::new(),
            api_url: String::new(),
            target_path: "/etc/cynic/secrets/forgejo-token".to_string(),
            mode: 0o640,
            scopes: vec![
                "write:repository".to_string(),
                "write:organization".to_string(),
                "write:admin".to_string(),
            ],
            expiry_seconds: Some(7_776_000),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenObservation {
    pub schema: String,
    pub principal: String,
    pub path: String,
    pub secret_present: bool,
    pub owner_root: Option<bool>,
    pub mode_o640: Option<bool>,
    pub api_authenticated: ForgejoTokenAuthState,
    pub unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenMetadata {
    pub token_id: u64,
    pub sha1: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenPlanAction {
    pub kind: ForgejoTokenActionKind,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenPlan {
    pub status: String,
    pub actions: Vec<ForgejoTokenPlanAction>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenStatusReport {
    pub schema: String,
    pub principal: String,
    pub token: ForgejoTokenObservation,
    pub forgejo_plan_status: String,
    pub token_action_needed: bool,
    pub requires_human_admin_secret: bool,
    pub plan_action: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenProvisionReport {
    pub schema: String,
    pub principal: String,
    pub status: String,
    pub metadata: Option<ForgejoTokenMetadata>,
    pub token_metadata: Option<ForgejoTokenMetadata>,
    pub action_taken: String,
    pub rollback_intent: String,
    pub unknowns: Vec<String>,
}
