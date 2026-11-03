//! Minimal scheduler configs owned by cynic-build so the build_scheduler
//! module has no cynic-node dependency.

use std::path::PathBuf;

/// Fields from `cynic-node`'s `BuildSchedulerConfig` that the scheduler
/// actually reads. The caller in cynic-node maps its full config onto this.
#[derive(Debug, Clone)]
pub struct BuildSchedulerConfig {
    pub registry_path: PathBuf,
    pub repository_id: String,
    pub transport_uri: Option<String>,
    pub interval_secs: u64,
    pub claim_ttl_secs: u64,
    pub command_timeout_secs: u64,
    pub sandbox_bwrap: bool,
    pub shared_target_root: Option<PathBuf>,
    pub stable_workspace_root: Option<PathBuf>,
    pub release_publication: Option<ReleasePublicationConfig>,
    pub build_jobs: Option<usize>,
    pub max_rss_cap_mb: Option<u64>,
    pub sccache: Option<SccacheConfig>,
    pub sccache_measured_hit_rate: Option<f64>,
    pub toolchain_bin_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SccacheConfig {
    pub cache_dir: PathBuf,
    pub binary: String,
    pub max_size_bytes: u64,
}

/// Fields from `cynic-node`'s `ReleasePublicationConfig` that the scheduler
/// actually reads.
#[derive(Debug, Clone)]
pub struct ReleasePublicationConfig {
    pub artifact_base_url: String,
    pub authentication_kind: String,
    pub work_root: PathBuf,
    pub signing_key_path: PathBuf,
    pub credential_path: PathBuf,
    pub signer_identity: String,
    pub signature_namespace: String,
    pub max_bytes: u64,
    pub timeout_secs: u64,
}

/// Fields from `cynic-node`'s `NodeControlConfig` that the scheduler
/// actually reads.
#[derive(Debug, Clone)]
pub struct NodeControlConfig {
    pub node_id: String,
    pub signing_key_path: PathBuf,
    pub agent_workload_state_root: PathBuf,
    pub runtime_artifact_max_bytes: u64,
    pub runtime_artifact_timeout_secs: u64,
    pub build_scheduler: Option<BuildSchedulerConfig>,
}
