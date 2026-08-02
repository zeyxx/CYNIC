use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use cynic_host::registry::DEFAULT_REGISTRY_PATH;
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BinaryAttestation {
    crate_version: &'static str,
    git_hash: &'static str,
    git_branch: &'static str,
    build_date: &'static str,
    executable_path: Option<String>,
    executable_sha256: Option<String>,
    executable_size_bytes: Option<u64>,
    executable_modified_unix_seconds: Option<u64>,
    runtime_artifact_declared: bool,
    declaration_status: &'static str,
    expected_publish_path: Option<String>,
    path_matches_expected: Option<bool>,
    runtime_artifact_proposal: Option<RuntimeArtifactProposal>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RuntimeArtifactProposal {
    name: &'static str,
    kind: &'static str,
    owner: &'static str,
    build_targets: Vec<&'static str>,
    publish_path: &'static str,
    primary_runtime_role: &'static str,
    rationale_sources: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RuntimeArtifactDeclarationDrift {
    pub(crate) runtime_artifact_name: &'static str,
    pub(crate) declaration_status: &'static str,
    pub(crate) drift_status: &'static str,
    pub(crate) executable_path: Option<String>,
    pub(crate) expected_publish_path: Option<String>,
    pub(crate) path_matches_expected: Option<bool>,
    pub(crate) evidence_sources: Vec<String>,
    pub(crate) proposed_runtime_artifact: Option<RuntimeArtifactProposal>,
}

#[derive(Debug, Deserialize)]
struct Registry {
    lifecycle: Option<RegistryLifecycle>,
}

#[derive(Debug, Deserialize)]
struct RegistryLifecycle {
    kernel_release: Option<KernelRelease>,
}

#[derive(Debug, Deserialize)]
struct KernelRelease {
    runtime_artifacts: Option<Vec<RuntimeArtifactDeclaration>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RuntimeArtifactDeclaration {
    Id(String),
    Detailed(RuntimeArtifact),
}

#[derive(Debug, Deserialize)]
struct RuntimeArtifact {
    name: Option<String>,
    publish: Option<RuntimePublish>,
}

#[derive(Debug, Deserialize)]
struct RuntimePublish {
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RuntimeBasket {
    artifacts: Vec<RuntimeBasketArtifact>,
}

#[derive(Debug, Deserialize)]
struct RuntimeBasketArtifact {
    id: String,
    install_path: String,
}

pub(crate) fn runtime_binary_attestation(registry_path: Option<&str>) -> BinaryAttestation {
    let executable_path = std::env::current_exe().ok();
    let executable_sha256 = executable_path
        .as_ref()
        .and_then(|path| sha256_file(path).ok());
    let metadata = executable_path
        .as_ref()
        .and_then(|path| fs::metadata(path).ok());
    let executable_size_bytes = metadata.as_ref().map(|meta| meta.len());
    let executable_modified_unix_seconds = metadata
        .as_ref()
        .and_then(|meta| meta.modified().ok())
        .and_then(system_time_to_unix_seconds);
    let expected_publish_path = expected_publish_path(registry_path);
    let path_matches_expected = executable_path.as_ref().and_then(|path| {
        expected_publish_path
            .as_ref()
            .map(|expected| path_to_string(path.clone()) == *expected)
    });
    let declaration_status = match (&expected_publish_path, path_matches_expected) {
        (None, _) => "missing_from_registry_runtime_artifacts",
        (Some(_), Some(true)) => "declared_and_path_matches",
        (Some(_), Some(false)) => "declared_but_path_mismatch",
        (Some(_), None) => "declared_path_unverifiable",
    };
    let runtime_artifact_proposal = if expected_publish_path.is_none() {
        Some(proposed_runtime_artifact())
    } else {
        None
    };

    BinaryAttestation {
        crate_version: env!("CARGO_PKG_VERSION"),
        git_hash: {
            let val = option_env!("CYNIC_BUILD_GIT_HASH").unwrap_or("unknown_hash");
            if val.is_empty() { "unknown_hash" } else { val }
        },
        git_branch: {
            let val = option_env!("CYNIC_BUILD_GIT_BRANCH").unwrap_or("unknown_branch");
            if val.is_empty() {
                "unknown_branch"
            } else {
                val
            }
        },
        build_date: {
            let val = option_env!("CYNIC_BUILD_DATE").unwrap_or("unknown_date");
            if val.is_empty() { "unknown_date" } else { val }
        },
        executable_path: executable_path.map(path_to_string),
        executable_sha256,
        executable_size_bytes,
        executable_modified_unix_seconds,
        runtime_artifact_declared: expected_publish_path.is_some(),
        declaration_status,
        expected_publish_path,
        path_matches_expected,
        runtime_artifact_proposal,
    }
}

impl BinaryAttestation {
    pub(crate) fn declaration_drift(&self) -> RuntimeArtifactDeclarationDrift {
        RuntimeArtifactDeclarationDrift {
            runtime_artifact_name: "cynic-node",
            declaration_status: self.declaration_status,
            drift_status: declaration_drift_status(
                self.runtime_artifact_declared,
                self.path_matches_expected,
            ),
            executable_path: self.executable_path.clone(),
            expected_publish_path: self.expected_publish_path.clone(),
            path_matches_expected: self.path_matches_expected,
            evidence_sources: vec![
                format!("{DEFAULT_REGISTRY_PATH}:lifecycle.kernel_release.runtime_artifacts"),
                "infra/systemd/cynic-node.service:ExecStart=/usr/local/bin/cynic-node".to_string(),
                "contracts/foundation.json:constitutional.runtime_authority_order".to_string(),
                "Cargo.toml:workspace.members includes crates/cynic-node".to_string(),
            ],
            proposed_runtime_artifact: self.runtime_artifact_proposal.clone(),
        }
    }
}

fn proposed_runtime_artifact() -> RuntimeArtifactProposal {
    RuntimeArtifactProposal {
        name: "cynic-node",
        kind: "rust_binary",
        owner: "cynic-node",
        build_targets: vec!["crates/cynic-node"],
        publish_path: "/usr/local/bin/cynic-node",
        primary_runtime_role: "all_runtime_nodes",
        rationale_sources: vec![
            "infra/systemd/cynic-node.service:ExecStart=/usr/local/bin/cynic-node",
            "contracts/foundation.json:constitutional.runtime_authority_order",
            "Cargo.toml:workspace.members includes crates/cynic-node",
        ],
    }
}

fn declaration_drift_status(
    runtime_artifact_declared: bool,
    path_matches_expected: Option<bool>,
) -> &'static str {
    match (runtime_artifact_declared, path_matches_expected) {
        (false, _) => "artifact_missing_from_registry",
        (true, Some(true)) => "declared_and_runtime_path_aligned",
        (true, Some(false)) => "declared_but_runtime_path_drifted",
        (true, None) => "declared_but_runtime_path_unverifiable",
    }
}

fn expected_publish_path(registry_path: Option<&str>) -> Option<String> {
    let path = registry_path?;
    let raw = fs::read_to_string(path).ok()?;
    let registry: Registry = serde_json::from_str(&raw).ok()?;
    let artifacts = registry.lifecycle?.kernel_release?.runtime_artifacts?;
    let publish = artifacts.into_iter().find_map(|artifact| match artifact {
        RuntimeArtifactDeclaration::Id(id) if id == "cynic-node" => {
            runtime_basket_install_path(&id)
        }
        RuntimeArtifactDeclaration::Detailed(artifact)
            if artifact.name.as_deref() == Some("cynic-node") =>
        {
            artifact.publish.and_then(|publish| publish.path)
        }
        _ => None,
    })?;
    Some(expand_home_placeholder(&publish))
}

fn runtime_basket_install_path(artifact_id: &str) -> Option<String> {
    let basket: RuntimeBasket =
        serde_json::from_str(include_str!("../../../infra/runtime-basket.json")).ok()?;
    basket
        .artifacts
        .into_iter()
        .find(|artifact| artifact.id == artifact_id)
        .map(|artifact| artifact.install_path)
}

fn sha256_file(path: &PathBuf) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let digest = Sha256::digest(bytes);
    Ok(hex::encode(digest))
}

fn system_time_to_unix_seconds(time: SystemTime) -> Option<u64> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().to_string()
}

fn expand_home_placeholder(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "%h".to_string());
    path.replace("%h", &home)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::{
        declaration_drift_status, expand_home_placeholder, expected_publish_path,
        runtime_binary_attestation,
    };

    #[test]
    fn runtime_attestation_includes_build_metadata() {
        let attestation = runtime_binary_attestation(None);

        assert!(!attestation.crate_version.is_empty());
        assert!(!attestation.git_hash.is_empty());
        assert!(!attestation.git_branch.is_empty());
        assert!(!attestation.build_date.is_empty());
    }

    #[test]
    fn attestation_proposes_runtime_artifact_when_registry_missing() {
        let attestation = runtime_binary_attestation(None);

        assert!(!attestation.runtime_artifact_declared);
        assert_eq!(
            attestation.declaration_status,
            "missing_from_registry_runtime_artifacts"
        );
        assert!(attestation.runtime_artifact_proposal.is_some());
        assert_eq!(
            attestation.declaration_drift().drift_status,
            "artifact_missing_from_registry"
        );
    }

    #[test]
    fn expands_home_placeholder() {
        let expected = std::env::var("HOME").unwrap_or_else(|_| "%h".to_string());
        assert_eq!(
            expand_home_placeholder("%h/bin/cynic-node"),
            format!("{expected}/bin/cynic-node")
        );
    }

    #[test]
    fn reads_expected_publish_path_from_registry_runtime_artifacts() {
        let mut file = tempfile::NamedTempFile::new().expect("temp registry");
        write!(
            file,
            r#"{{
              "lifecycle": {{
                "kernel_release": {{
                    "runtime_artifacts": [
                    {{
                      "name": "cynic-node",
                      "publish": {{
                        "path": "/usr/local/bin/cynic-node"
                      }}
                    }}
                  ]
                }}
              }}
            }}"#
        )
        .expect("write registry");

        assert_eq!(
            expected_publish_path(Some(&file.path().to_string_lossy())),
            Some("/usr/local/bin/cynic-node".to_string())
        );
    }

    #[test]
    fn resolves_compact_registry_artifact_through_runtime_basket() {
        let mut file = tempfile::NamedTempFile::new().expect("temp registry");
        write!(
            file,
            r#"{{
              "lifecycle": {{
                "kernel_release": {{
                  "runtime_artifacts": ["cynic", "cynic-node"]
                }}
              }}
            }}"#
        )
        .expect("write registry");

        assert_eq!(
            expected_publish_path(Some(&file.path().to_string_lossy())),
            Some("/usr/local/bin/cynic-node".to_string())
        );
    }

    #[test]
    fn declaration_drift_status_maps_expected_states() {
        assert_eq!(
            declaration_drift_status(false, None),
            "artifact_missing_from_registry"
        );
        assert_eq!(
            declaration_drift_status(true, Some(true)),
            "declared_and_runtime_path_aligned"
        );
        assert_eq!(
            declaration_drift_status(true, Some(false)),
            "declared_but_runtime_path_drifted"
        );
        assert_eq!(
            declaration_drift_status(true, None),
            "declared_but_runtime_path_unverifiable"
        );
    }
}
