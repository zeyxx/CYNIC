//! Driven ports of the build bounded context (hexagonal architecture).
//!
//! The domain (types.rs, runner.rs) depends only on these traits. Adapters
//! in adapters/ provide the OS-backed implementations; tests and downstream
//! crates provide fakes. No type in this file performs I/O.

use std::path::Path;
use std::path::PathBuf;

use crate::SignedBuildWorkloadGrant;
use crate::build_proof::{BuildProofReport, BuildProofRequest};

use async_trait::async_trait;

/// Port for executing build commands (dependency inversion).
pub trait BuildRunner: Send + Sync {
    fn run_command(&self, argv: &[&str]) -> BuildCommandOutput;
}

/// Value object returned through the [`BuildRunner`] port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub spawn_error: Option<String>,
}

impl BuildCommandOutput {
    pub fn success(&self) -> bool {
        self.spawn_error.is_none() && self.exit_code == Some(0)
    }

    pub fn summary(&self) -> String {
        if let Some(err) = &self.spawn_error {
            return format!("spawn error: {err}");
        }
        let code = self
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "none".into());
        let msg = if !self.stdout.trim().is_empty() {
            self.stdout.trim()
        } else if !self.stderr.trim().is_empty() {
            self.stderr.trim()
        } else {
            "empty output"
        };
        format!("exit {code}: {msg}")
    }
}

/// Port for the filesystem effects of the build pipeline: artifact
/// read-back and atomic installation with rollback.
///
/// Errors are opaque strings so adapters never leak OS error types into
/// the domain. Paths stay as [`Path`] so the domain never formats them.
pub trait FilesystemPort: Send + Sync {
    /// Readiness error type is folded into strings at the adapter edge.
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String>;
    fn copy(&self, source: &Path, target: &Path) -> Result<(), String>;
    fn write_bytes(&self, path: &Path, bytes: &[u8]) -> Result<(), String>;
    fn rename(&self, source: &Path, target: &Path) -> Result<(), String>;
    fn remove_file(&self, path: &Path) -> Result<(), String>;
    fn exists(&self, path: &Path) -> bool;
}

/// Port for process-identity observations the pipeline must not shell out
/// to obtain (no `id -u` subprocess, no ambient uid reads in the domain).
pub trait IdentityPort: Send + Sync {
    fn uid(&self) -> u32;
    fn pid(&self) -> u32;
}


/// Port for signing build workload grants.
pub trait BuildWorkloadGrantSignerPort: Send + Sync {
    fn authority_key_id(&self) -> &str;
    fn sign(&self, payload: &[u8]) -> Result<String, String>;
}

/// Port for verifying build workload grants.
pub trait BuildWorkloadGrantVerifierPort: Send + Sync {
    fn verify(
        &self,
        authority_key_id: &str,
        payload: &[u8],
        signature_hex: &str,
    ) -> Result<(), String>;
}

/// Error returned by build workload grant store operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildWorkloadGrantStoreError {
    pub code: String,
    pub detail: String,
}

impl BuildWorkloadGrantStoreError {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

/// A persisted build workload grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedBuildWorkloadGrant {
    pub grant: SignedBuildWorkloadGrant,
    pub evidence_ref: String,
    pub path: PathBuf,
}

/// Port for storing and retrieving build workload grants.
pub trait BuildWorkloadGrantStorePort: Send + Sync {
    fn latest(
        &self,
        workload_id: &str,
    ) -> Result<Option<PersistedBuildWorkloadGrant>, BuildWorkloadGrantStoreError>;

    fn persist(
        &self,
        grant: &SignedBuildWorkloadGrant,
    ) -> Result<PersistedBuildWorkloadGrant, BuildWorkloadGrantStoreError>;
}

#[async_trait]
pub trait BuildProofPort: Send + Sync {
    async fn prove(&self, request: &BuildProofRequest) -> Result<BuildProofReport, String>;

    fn node_id(&self) -> &str;
}

use std::future::Future;
use std::pin::Pin;

use crate::release_publication::{RuntimeReleasePublicationManifest, SignedReleaseBasketPlan};
use cynic_protocol::RuntimeArtifactSource;

#[derive(Debug)]
pub struct PackagedReleaseBasket {
    pub path: std::path::PathBuf,
    pub digest: String,
    pub size_bytes: u64,
    pub(crate) _retention: tempfile::TempDir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetachedReleaseSignature {
    pub path: std::path::PathBuf,
    pub digest: String,
    pub size_bytes: u64,
    pub signer_identity: String,
    pub namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePublicationTargets {
    pub basket: RuntimeArtifactSource,
    pub detached_signature: RuntimeArtifactSource,
    pub manifest: RuntimeArtifactSource,
}

pub type ReleasePublicationFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

pub trait ReleaseBasketPackagingPort: Send + Sync {
    fn package<'a>(
        &'a self,
        plan: &'a SignedReleaseBasketPlan,
        workspace_root: &'a std::path::Path,
        workload_id: &'a str,
    ) -> ReleasePublicationFuture<'a, PackagedReleaseBasket>;
}

pub trait ReleaseBasketSigningPort: Send + Sync {
    fn sign<'a>(
        &'a self,
        basket: &'a PackagedReleaseBasket,
    ) -> ReleasePublicationFuture<'a, DetachedReleaseSignature>;
}

pub trait ReleaseArtifactPublicationPort: Send + Sync {
    fn targets(&self, release_id: &str) -> Result<ReleasePublicationTargets, String>;

    fn publish<'a>(
        &'a self,
        node_id: &'a str,
        workload_id: &'a str,
        basket: &'a PackagedReleaseBasket,
        signature: &'a DetachedReleaseSignature,
        manifest: &'a RuntimeReleasePublicationManifest,
        targets: &'a ReleasePublicationTargets,
        observed_at_ms: u64,
    ) -> ReleasePublicationFuture<'a, crate::SignedReleaseBasketReadBack>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePublicationError {
    pub code: String,
    pub detail: String,
}

impl ReleasePublicationError {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for ReleasePublicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for ReleasePublicationError {}

pub type ReleasePublicationResult<T> = Result<T, ReleasePublicationError>;

// --- Hoisted from cynic-node: build scheduler ports ---

use std::collections::BTreeMap;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use cynic_agent::{AgentWorkspaceLayout, AgentWorkspacePortError};
use cynic_sandbox::cargo::BuildSandboxMode;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BuildWorkspaceReadBack {
    pub workspace_root: PathBuf,
    pub source_commit: String,
    pub created: bool,
}

pub trait BuildWorkspacePort: Send + Sync {
    fn prepare(
        &self,
        layout: &AgentWorkspaceLayout,
        source_commit: &str,
    ) -> Result<BuildWorkspaceReadBack, AgentWorkspacePortError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildExecutionRequest {
    pub argv: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub timeout: Duration,
    /// Isolation mode. `Off` runs the admitted argv as a bare subprocess;
    /// `Bwrap` confines it with Bubblewrap and fails closed when the
    /// helper is unavailable.
    pub sandbox: BuildSandboxMode,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BuildExecutionReadBack {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// Peak resident memory observed for the child process, in bytes.
    /// None means the process memory could not be observed.
    #[serde(default)]
    pub peak_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildExecutionError {
    Invalid(String),
    Spawn(String),
    Timeout,
    Cancelled,
}

pub trait BuildExecutionPort: Send + Sync {
    fn execute<'a>(
        &'a self,
        request: &'a BuildExecutionRequest,
        cancellation: &'a CancellationToken,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<BuildExecutionReadBack, BuildExecutionError>>
                + Send
                + 'a,
        >,
    >;
}

// --- Additional build-scheduler ports ---

/// Port for staging runtime artifacts (e.g. source snapshots).
#[async_trait::async_trait]
pub trait RuntimeArtifactTransportPort: Send + Sync {
    async fn stage(
        &self,
        source: &RuntimeArtifactSource,
        destination: &std::path::Path,
        expected_digest: Option<&str>,
    ) -> Result<cynic_protocol::artifact_transport::ArtifactTransferReceipt, String>;
}

/// Capacity tier outcome for the admission gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadTier {
    Headroom,
    Normal,
    Saturated,
}

#[derive(Debug, Clone)]
pub struct CapacityAssessment {
    pub state: LoadTier,
    pub residual_unknowns: Vec<String>,
}

/// Port for observing host capacity. The scheduler calls this once per tick.
pub trait CapacityAssessmentPort: Send + Sync {
    fn assess(&self, policy: &LoadPolicy) -> CapacityAssessment;
}

#[derive(Debug, Clone)]
pub struct LoadPolicy {
    pub domain: String,
}

impl LoadPolicy {
    pub fn with_legacy_defaults(domain: impl Into<String>) -> Self {
        Self { domain: domain.into() }
    }
}

/// Opaque CPU sample token.
#[derive(Debug, Clone, Copy)]
pub struct CpuSample;

/// Port for bracketing CPU measurement around a build scope.
pub trait CpuMeterPort: Send + Sync {
    fn mark(&self) -> CpuSample;
    fn cpu_ms_between(&self, _start: &CpuSample, _end: &CpuSample) -> Option<u64>;
}

/// Port for resolving workspace layouts from a runtime registry.
pub trait WorkspaceLayoutResolver: Send + Sync {
    fn resolve(
        &self,
        repository_id: &str,
        workload_id: &str,
        node_id: &str,
    ) -> Result<AgentWorkspaceLayout, AgentWorkspacePortError>;
}

/// Recompute the evidence digest for a resolved layout after a path override.
pub fn recompute_layout_digest(
    layout: &AgentWorkspaceLayout,
    node_id: &str,
    repository_id: &str,
    workspace_id: &str,
) -> Result<String, String> {
    use cynic_protocol::digest_json;
    digest_json(&(
        node_id,
        repository_id,
        workspace_id,
        layout.repository_root.clone(),
        layout.workspace_root.clone(),
        layout.dedicated,
        &layout.required_principal,
    ))
    .map_err(|error| error.to_string())
}

/// Port for reclaiming build artefacts before a build consumes space.
pub trait BuildArtefactReclaimer: Send + Sync {
    fn reclaim(&self);
}
