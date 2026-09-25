use serde::{Deserialize, Serialize};

pub const RUNTIME_ARTIFACT_RELEASE_SPEC_SCHEMA: &str = "cynic.runtime_artifact_release.spec.v1";
pub const RUNTIME_ARTIFACT_RELEASE_REPORT_SCHEMA: &str = "cynic.runtime_artifact_release.report.v1";
pub const RUNTIME_ARTIFACT_RELEASE_EVENT_SCHEMA: &str = "cynic.runtime_artifact_release.event.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Optional remote publication target. When present, the CLI publishes
    /// the staged artifact to this target after a successful local
    /// activation (provenance: artifact sha256 + git_sha). Absent means
    /// local-only release (historical behavior, unchanged).
    #[serde(default)]
    pub publication: Option<ArtifactPublicationTarget>,
}

/// Remote publication target for a released artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactPublicationTarget {
    /// Registry node id owning the target (e.g. `synapse`).
    pub node: String,
    /// Concrete endpoint for the transport (host, URL, or ssh target).
    pub endpoint: String,
    /// Transport protocol: `http_put` (default) or `ssh`.
    #[serde(default = "default_publication_protocol")]
    pub protocol: String,
}

fn default_publication_protocol() -> String {
    "http_put".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_spec() -> serde_json::Value {
        serde_json::json!({
            "schema": "cynic.runtime_artifact_release.spec.v1",
            "node_id": "n",
            "phase": "conformance",
            "service_id": "s",
            "identity_schema": "i",
            "artifact": {
                "id": "a", "path": "/tmp/a", "executable_name": "a",
                "sha256": "b", "version": "0.2.0", "git_sha": "c"
            },
            "install_root": "/var/lib/cynic",
            "identity_url": "http://127.0.0.1:9090/identity",
            "restart_argv": ["systemctl", "restart"],
            "requires_privilege": false
        })
    }

    #[test]
    fn legacy_spec_without_publication_stays_local_only() {
        let spec: RuntimeArtifactReleaseSpec =
            serde_json::from_value(legacy_spec()).expect("legacy spec parses");
        assert!(spec.publication.is_none());
    }

    #[test]
    fn publication_protocol_defaults_to_http_put() {
        let mut raw = legacy_spec();
        raw["publication"] = serde_json::json!({"node": "synapse", "endpoint": ""});
        let spec: RuntimeArtifactReleaseSpec =
            serde_json::from_value(raw).expect("spec with publication parses");
        let target = spec.publication.expect("target");
        assert_eq!(target.protocol, "http_put");
    }
}
