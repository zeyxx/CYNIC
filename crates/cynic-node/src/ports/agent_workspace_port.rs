use std::path::PathBuf;

use serde::Serialize;

use cynic_protocol::{
    AgentWorkspaceClaimAdmission, AgentWorkspaceClaimIntent, AgentWorkspaceClaimObservation,
    AgentWorkspaceClaimOutcome, AgentWorkspaceLifecycleObservation, AgentWorkspaceLifecyclePlan,
    AgentWorkspaceLifecycleRecord, AgentWorkspaceObservation, AgentWorkspaceRequest, LeaseGuard,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentWorkspaceLayout {
    pub repository_root: PathBuf,
    pub workspace_root: PathBuf,
    pub dedicated: bool,
    pub required_principal: String,
    pub evidence_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentWorkspacePortError {
    pub code: String,
    pub message: String,
}

impl AgentWorkspacePortError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub trait AgentWorkspaceLayoutPort: Send + Sync {
    fn resolve(
        &self,
        repository_id: &str,
        workspace_id: &str,
        node_id: &str,
    ) -> Result<AgentWorkspaceLayout, AgentWorkspacePortError>;
}

/// Read-only substrate observer. Repository claims are supplied by the durable
/// claim store so filesystem/Git observation cannot silently invent authority.
pub trait AgentWorkspaceObservationPort: Send + Sync {
    fn observe(
        &self,
        request: &AgentWorkspaceRequest,
        repository_claims: Vec<AgentWorkspaceClaimObservation>,
    ) -> Result<AgentWorkspaceObservation, AgentWorkspacePortError>;
}

/// Bounded Git worktree creation boundary. The provisioner may create only the
/// exact workspace resolved by the layout port at the requested base commit.
/// Ownership and HEAD remain observation concerns and must be read back through
/// `AgentWorkspaceObservationPort` before reconciliation can admit source work.
pub trait AgentWorkspaceProvisionPort: Send + Sync {
    fn ensure(
        &self,
        layout: &AgentWorkspaceLayout,
        request: &AgentWorkspaceRequest,
    ) -> Result<bool, AgentWorkspacePortError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentWorkspaceCollectionReadBack {
    pub workspace_existed_before: bool,
    pub workspace_exists_after: bool,
    pub reclaimed_bytes: Option<u64>,
    pub completion_evidence_digest: String,
}

/// Bounded collection boundary for an expired, dedicated agent workspace.
///
/// Implementations must verify the exact layout and fresh collection guard,
/// remove the Git worktree through argv execution, then read back absence and
/// measured capacity. A command exit status alone is not collection proof.
pub trait AgentWorkspaceCollectionPort: Send + Sync {
    fn collect(
        &self,
        layout: &AgentWorkspaceLayout,
        plan: &AgentWorkspaceLifecyclePlan,
    ) -> Result<AgentWorkspaceCollectionReadBack, AgentWorkspacePortError>;
}

/// Append-once node-owned completion, retention, and protected-root store.
///
/// A workload id is immutable: persisting the same record is idempotent, while
/// a different record for an existing workload must be rejected.
pub trait AgentWorkspaceLifecycleStorePort: Send + Sync {
    fn load(
        &self,
        workload_id: &str,
    ) -> Result<Option<AgentWorkspaceLifecycleRecord>, AgentWorkspacePortError>;

    fn persist(
        &self,
        record: &AgentWorkspaceLifecycleRecord,
    ) -> Result<AgentWorkspaceLifecycleRecord, AgentWorkspacePortError>;
}

/// Read-only terminal lifecycle observer. Durable completion and retention are
/// supplied by the lifecycle store; Git state and claims are observed live.
pub trait AgentWorkspaceTerminalObservationPort: Send + Sync {
    fn observe_terminal(
        &self,
        layout: &AgentWorkspaceLayout,
        record: &AgentWorkspaceLifecycleRecord,
        repository_claims: Vec<AgentWorkspaceClaimObservation>,
    ) -> Result<AgentWorkspaceLifecycleObservation, AgentWorkspacePortError>;
}

/// Durable, node-owned authority for hierarchical repository zones.
///
/// `reserve` must first verify `workspace_claim_admission_matches`, then compare
/// all live claims and persist the new grant atomically.
/// `renew` and `release` must reject stale fencing tokens. An implementation
/// must never emulate this contract with a check followed by an unrelated write.
pub trait AgentWorkspaceClaimStorePort: Send + Sync {
    async fn reserve(
        &self,
        intent: &AgentWorkspaceClaimIntent,
        admission: &AgentWorkspaceClaimAdmission,
        now_ms: u64,
    ) -> Result<AgentWorkspaceClaimOutcome, AgentWorkspacePortError>;

    async fn repository_claims(
        &self,
        repository_id: &str,
        now_ms: u64,
    ) -> Result<Vec<AgentWorkspaceClaimObservation>, AgentWorkspacePortError>;

    async fn renew(
        &self,
        guard: &LeaseGuard,
        requested_ttl_ms: u64,
        now_ms: u64,
    ) -> Result<AgentWorkspaceClaimObservation, AgentWorkspacePortError>;

    async fn release(&self, guard: &LeaseGuard, now_ms: u64)
        -> Result<(), AgentWorkspacePortError>;

    async fn finalize(
        &self,
        guard: &LeaseGuard,
        proof_ref: &str,
        now_ms: u64,
    ) -> Result<(), AgentWorkspacePortError>;
}
