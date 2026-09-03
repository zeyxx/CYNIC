//! Build runner port — agent-facing trigger for the distributed CYNIC build.
//!
//! This is the missing interface the agents needed: instead of invoking
//! `cargo` ad hoc on a local machine (with stack-size hacks to dodge toolchain
//! ICEs), an agent triggers a build on the network build runner (10.10.10.119)
//! through this port. The runner materializes a clean detached workspace,
//! runs `cynic-build-pipeline`, and returns a signed proof bundle
//! (`cynic.pipeline_build_proof.v1`). The build becomes a side-effect of a
//! commit/PR, not a manual local invocation.
//!
//! Design (CYNIC coherence pattern):
//!   desired   -> BuildTrigger (exact source commit + phase)
//!   observe   -> RunnerHealth (toolchain, cache warmth, liveness)
//!   reconcile -> the runner decides admission + layer order (already in the
//!                pipeline script; the port is a thin projection of it)
//!   bounded   -> --source-commit (exact 40-char) + --target-phase + --yes
//!   read-back -> BuildProofBundle (per-layer status, aggregated convergence)

use std::fmt;

/// Build phase requested on the runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildPhase {
    /// `cargo check` only — fast convergence signal.
    Check,
    /// Full release build of runtime artifacts.
    Build,
    /// Build + activate (enroll node, install service).
    Activate,
}

impl BuildPhase {
    /// CLI / argv token for the runner's `--target-phase`.
    pub fn as_arg(&self) -> &'static str {
        match self {
            BuildPhase::Check => "check",
            BuildPhase::Build => "build",
            BuildPhase::Activate => "activate",
        }
    }
}

impl fmt::Display for BuildPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_arg())
    }
}

/// Network mode the runner may use to resolve dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    Offline,
    Online,
}

impl NetworkMode {
    pub fn as_arg(&self) -> &'static str {
        match self {
            NetworkMode::Offline => "offline",
            NetworkMode::Online => "online",
        }
    }
}

/// A build request: an exact source commit and the phase to run.
///
/// The exact 40-char commit is the desired state — the runner materializes a
/// clean detached workspace from it, so no mutable local tree can be admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildTrigger {
    pub source_commit: String,
    pub phase: BuildPhase,
    pub network_mode: NetworkMode,
}

impl BuildTrigger {
    /// Validate the commit is an exact 40-char hex SHA — fail-closed before
    /// any runner round-trip.
    pub fn validate(&self) -> Result<(), BuildError> {
        if !self.source_commit.chars().all(|c| c.is_ascii_hexdigit())
            || self.source_commit.len() != 40
        {
            return Err(BuildError::InvalidCommit(self.source_commit.clone()));
        }
        Ok(())
    }
}

/// Per-layer build result, mirroring `cynic.layer_build_proof.v1`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LayerProof {
    pub layer: String,
    pub status: LayerStatus,
    /// If blocked, the upstream layer that blocked it.
    pub blocked_by: Option<String>,
}

/// Status of a single layer in the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LayerStatus {
    /// Layer check/test passed.
    Converged,
    /// Layer failed its own check/test.
    Failed,
    /// Layer not attempted because an upstream layer blocked it.
    BlockedUpstream,
}

/// Aggregated pipeline proof, mirroring `cynic.pipeline_build_proof.v1`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BuildProofBundle {
    pub schema: String,
    pub node: String,
    pub source_commit: String,
    pub action: String,
    pub status: PipelineStatus,
    pub broken_layer: Option<String>,
    pub layers: Vec<LayerProof>,
}

/// Whole-pipeline convergence status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PipelineStatus {
    Converged,
    Blocked,
}

impl BuildProofBundle {
    /// Machine-readable JSON projection (the read-back proof surface).
    pub fn to_json(&self) -> String {
        // Hand-rolled to avoid a serde dependency in this port module; the
        // shapes are small and stable.
        let layers: Vec<String> = self
            .layers
            .iter()
            .map(|l| {
                format!(
                    "{{\"layer\":\"{}\",\"status\":\"{}\",\"blocked_by\":{}}}",
                    l.layer,
                    match l.status {
                        LayerStatus::Converged => "converged",
                        LayerStatus::Failed => "failed",
                        LayerStatus::BlockedUpstream => "blocked_upstream",
                    },
                    match &l.blocked_by {
                        Some(b) => format!("\"{}\"", b),
                        None => "null".to_string(),
                    }
                )
            })
            .collect();
        format!(
            "{{\"schema\":\"{}\",\"node\":\"{}\",\"source_commit\":\"{}\",\"action\":\"{}\",\"status\":\"{}\",\"broken_layer\":{},\"layers\":[{}]}}",
            self.schema,
            self.node,
            self.source_commit,
            self.action,
            match self.status {
                PipelineStatus::Converged => "converged",
                PipelineStatus::Blocked => "blocked",
            },
            match &self.broken_layer {
                Some(b) => format!("\"{}\"", b),
                None => "null".to_string(),
            },
            layers.join(",")
        )
    }
}

/// Health of the network build runner, observed before triggering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerHealth {
    pub node_id: String,
    pub reachable: bool,
    /// Whether the runner has a warm incremental cache for the requested commit.
    pub cache_warm: bool,
    pub toolchain: String,
    /// Source SHA the runner binary reports via `cynic version --json`.
    /// None when the runner binary predates the version surface or the
    /// query fails: staleness unknown, never assumed fresh.
    pub binary_sha: Option<String>,
}

/// Errors from the build runner port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    InvalidCommit(String),
    RunnerUnreachable(String),
    AdmissionBlocked(String),
    ProofUnparseable(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::InvalidCommit(c) => write!(f, "invalid source commit: {c}"),
            BuildError::RunnerUnreachable(e) => write!(f, "runner unreachable: {e}"),
            BuildError::AdmissionBlocked(e) => write!(f, "build admission blocked: {e}"),
            BuildError::ProofUnparseable(e) => write!(f, "build proof unparseable: {e}"),
        }
    }
}

impl std::error::Error for BuildError {}

/// Port: trigger a distributed build and observe runner health.
///
/// The adapter (ssh / API) is disposable; this contract is durable. An agent
/// surface (MCP tool `cynic_build_trigger`, CLI `cynic build`) consumes the
/// port — it never shells out to `cargo` directly.
pub trait BuildRunnerPort {
    /// Trigger a build on the runner and return its proof bundle.
    fn trigger(&self, trigger: &BuildTrigger) -> Result<BuildProofBundle, BuildError>;
    /// Observe runner health before triggering (liveness, cache, toolchain).
    fn observe_runner(&self) -> RunnerHealth;
}
