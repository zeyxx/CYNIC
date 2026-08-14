use serde::{Deserialize, Serialize};

pub const RUNTIME_ARTIFACT_RELEASE_SPEC_SCHEMA: &str = "cynic.runtime_artifact_release.spec.v1";
pub const RUNTIME_ARTIFACT_RELEASE_REPORT_SCHEMA: &str = "cynic.runtime_artifact_release.report.v1";
pub const RUNTIME_ARTIFACT_RELEASE_EVENT_SCHEMA: &str = "cynic.runtime_artifact_release.event.v1";

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeArtifactReleaseSpec {
    pub schema: String,
    pub node_id: String,
    pub phase: String,
    pub service_id: String,
    pub identity_schema: String,
    pub artifact: RuntimeArtifactSpec,
    pub install_root: String,
    pub identity_url: String,
    pub restart_argv: Vec<String>,
    #[serde(default)]
    pub requires_privilege: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeArtifactSpec {
    pub id: String,
    pub path: String,
    pub executable_name: String,
    pub sha256: String,
    pub version: String,
    pub git_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceIdentityEvidence {
    pub schema: String,
    pub service: String,
    pub node_id: String,
    pub hostname: String,
    pub pid: u32,
    pub build: RuntimeBuildEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeBuildEvidence {
    pub version: String,
    pub git_sha: String,
    pub profile: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeArtifactReleaseAction {
    pub phase: &'static str,
    pub description: String,
    pub mutates: bool,
    pub requires_privilege: bool,
    pub proof: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeArtifactReleaseReport {
    pub schema: &'static str,
    pub status: String,
    pub artifact_id: String,
    pub service_id: String,
    pub node_id: String,
    pub lifecycle_phase: String,
    pub mutates: bool,
    pub desired_sha256: String,
    pub desired_version: String,
    pub desired_git_sha: String,
    pub previous_target: Option<String>,
    pub activated_target: Option<String>,
    pub observed_before: Option<ServiceIdentityEvidence>,
    pub observed_after: Option<ServiceIdentityEvidence>,
    pub rolled_back: bool,
    pub actions: Vec<RuntimeArtifactReleaseAction>,
    pub proof_event: Option<RuntimeArtifactReleaseEvent>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeArtifactReleaseEvent {
    pub schema: String,
    pub seq: u64,
    pub timestamp_unix_ms: u64,
    pub artifact_id: String,
    pub service_id: String,
    pub node_id: String,
    pub lifecycle_phase: String,
    pub event: String,
    pub desired_sha256: String,
    pub previous_target: Option<String>,
    pub activated_target: Option<String>,
    pub observed_version: Option<String>,
    pub observed_git_sha: Option<String>,
    pub status: String,
    pub prev_hash: String,
    pub hash: String,
}
