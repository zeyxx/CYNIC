use std::path::Path;

use cynic_protocol::{
    PRE_OS_PLATFORM_OBSERVATION_SCHEMA, PreOsArtifactManifest, PreOsControlKind,
    PreOsPlatformObservation, SecurityEvidenceState,
};
use sha2::{Digest, Sha256};

use crate::ports::{
    PreOsArtifactObservationPort, PreOsAttestationObservationPort, PreOsPlatformObservationPort,
};
use crate::pre_os_control::{PreOsArtifactObservation, PreOsAttestationSinkObservation};
use crate::process_observation::command_exit_success;
use crate::runtime_layout::current_runtime_layout;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalPreOsAdapter;

impl PreOsPlatformObservationPort for LocalPreOsAdapter {
    fn observe_pre_os_platform(&self) -> PreOsPlatformObservation {
        let uefi_variables = Path::new("/sys/firmware/efi/efivars").is_dir();
        let boot_once_tool =
            command_exit_success(&current_runtime_layout().which_bin(), &["efibootmgr"])
                .unwrap_or(false);
        let mut controls = Vec::new();
        let mut evidence_refs = Vec::new();
        if uefi_variables {
            controls.push(PreOsControlKind::UefiVariables);
            evidence_refs.push("linux-sysfs:efi-variables".to_string());
        }
        if boot_once_tool {
            evidence_refs.push("argv-capability:efibootmgr".to_string());
        }
        PreOsPlatformObservation {
            schema: PRE_OS_PLATFORM_OBSERVATION_SCHEMA.to_string(),
            platform: std::env::consts::OS.to_string(),
            controls,
            boot_once: if uefi_variables && boot_once_tool {
                SecurityEvidenceState::Proven
            } else if uefi_variables {
                SecurityEvidenceState::Declared
            } else {
                SecurityEvidenceState::Unknown
            },
            artifact_discovery: SecurityEvidenceState::Unknown,
            attestation_sink: SecurityEvidenceState::Unknown,
            evidence_refs,
        }
    }
}

impl PreOsArtifactObservationPort for LocalPreOsAdapter {
    fn observe_pre_os_artifacts(&self) -> Vec<PreOsArtifactObservation> {
        let Some(path) = std::env::var_os("CYNIC_PRE_OS_ARTIFACT_MANIFEST") else {
            return Vec::new();
        };
        let Ok(payload) = std::fs::read(path) else {
            return Vec::new();
        };
        let Ok(manifest) = serde_json::from_slice::<PreOsArtifactManifest>(&payload) else {
            return Vec::new();
        };
        if !manifest.validate().is_empty() {
            return Vec::new();
        }
        manifest
            .artifacts
            .into_iter()
            .map(|artifact| {
                let integrity = verify_local_artifact(&artifact.source_ref, &artifact.digest);
                PreOsArtifactObservation {
                    harness_id: artifact.harness_id,
                    digest: Some(artifact.digest),
                    integrity,
                    source: "injected_manifest".to_string(),
                }
            })
            .collect()
    }
}

impl PreOsAttestationObservationPort for LocalPreOsAdapter {
    fn observe_pre_os_attestation_sink(&self) -> PreOsAttestationSinkObservation {
        let configured = std::env::var_os("CYNIC_PRE_OS_ATTESTATION_DIR")
            .is_some_and(|path| Path::new(&path).is_dir());
        PreOsAttestationSinkObservation {
            kind: if configured {
                "local_content_addressed_directory"
            } else {
                "unconfigured"
            }
            .to_string(),
            readiness: if configured {
                SecurityEvidenceState::Declared
            } else {
                SecurityEvidenceState::Unknown
            },
            evidence_refs: if configured {
                vec!["env:CYNIC_PRE_OS_ATTESTATION_DIR".to_string()]
            } else {
                Vec::new()
            },
        }
    }
}

fn verify_local_artifact(source_ref: &str, expected_digest: &str) -> SecurityEvidenceState {
    let Some(path) = source_ref.strip_prefix("file:") else {
        return SecurityEvidenceState::Declared;
    };
    let Ok(payload) = std::fs::read(path) else {
        return SecurityEvidenceState::Absent;
    };
    let observed = format!("sha256:{}", hex::encode(Sha256::digest(payload)));
    if observed == expected_digest {
        SecurityEvidenceState::Proven
    } else {
        SecurityEvidenceState::Refuted
    }
}

#[cfg(test)]
mod tests {
    use super::verify_local_artifact;
    use cynic_protocol::SecurityEvidenceState;

    #[test]
    fn non_file_provider_remains_declared_for_another_adapter() {
        assert_eq!(
            verify_local_artifact("https://boot.example/memtest.efi", "sha256:any"),
            SecurityEvidenceState::Declared
        );
    }
}
