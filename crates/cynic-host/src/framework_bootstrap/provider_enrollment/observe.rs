use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::runtime_secret_install::observe_runtime_secret;

use super::types::{ProviderControlEnrollmentObservation, ProviderControlEnrollmentSpec};

#[derive(Debug, Default, Deserialize)]
struct InstalledProviderConfiguration {
    endpoint: Option<String>,
    token_id: Option<String>,
    token_secret_file: Option<String>,
    ca_file: Option<String>,
}

pub fn observe_provider_control_enrollment(
    spec: &ProviderControlEnrollmentSpec,
) -> ProviderControlEnrollmentObservation {
    let mut unknowns = Vec::new();
    let config_metadata = metadata(&spec.config_target, "provider_config", &mut unknowns);
    let config = config_metadata
        .as_ref()
        .filter(|metadata| metadata.file_type().is_file() && !metadata.file_type().is_symlink())
        .and_then(|_| fs::read_to_string(&spec.config_target).ok())
        .and_then(|content| toml::from_str::<InstalledProviderConfiguration>(&content).ok());
    let secret_source = secret_source_admissible(&spec.token_secret_source, 64 * 1024);
    let secret_target = observe_runtime_secret(&spec.secret);
    let ca_source_present = spec.ca_source.as_ref().map(|path| path.is_file());
    let ca_source_admissible = spec
        .ca_source
        .as_ref()
        .map(|path| public_source_admissible(path, 1024 * 1024));
    let ca_target_metadata = metadata(&spec.ca_target, "provider_ca", &mut unknowns);
    let ca_digest_matches = spec.ca_source.as_ref().map(|source| {
        file_digest(source)
            .zip(file_digest(&spec.ca_target))
            .is_some_and(|(source, target)| source == target)
    });

    ProviderControlEnrollmentObservation {
        schema: "cynic.provider_control.enrollment_observation.v1",
        provider: spec.desired.provider.clone(),
        config_path: spec.config_target.display().to_string(),
        config_present: config_metadata.is_some(),
        config_regular_file: config_metadata
            .as_ref()
            .map(|metadata| metadata.file_type().is_file() && !metadata.file_type().is_symlink()),
        config_mode: config_metadata
            .as_ref()
            .map(|metadata| metadata.permissions().mode() & 0o777),
        config_owner_id: config_metadata.as_ref().map(MetadataExt::uid),
        config_group_id: config_metadata.as_ref().map(MetadataExt::gid),
        endpoint_matches: config
            .as_ref()
            .map(|config| config.endpoint.as_deref() == Some(spec.desired.endpoint.as_str())),
        token_identity_matches: config
            .as_ref()
            .map(|config| config.token_id.as_deref() == Some(spec.desired.token_identity.as_str())),
        token_secret_path_matches: config
            .as_ref()
            .map(|config| config.token_secret_file.as_deref() == spec.secret.target.to_str()),
        ca_path_matches: config.as_ref().map(|config| match spec.ca_source.as_ref() {
            Some(_) => config.ca_file.as_deref() == spec.ca_target.to_str(),
            None => config.ca_file.is_none(),
        }),
        secret_source_present: spec.token_secret_source.is_file(),
        secret_source_admissible: secret_source,
        secret_target_present: secret_target.present,
        secret_target_admissible: secret_target.present
            && secret_target.regular_file == Some(true)
            && secret_target.symlink == Some(false)
            && secret_target.owner_id == Some(spec.secret.owner_id)
            && secret_target.group_id == spec.secret.group_id
            && secret_target.mode == Some(spec.secret.mode)
            && secret_target.non_empty == Some(true),
        ca_source_present,
        ca_source_admissible,
        ca_target_present: ca_target_metadata.is_some(),
        ca_target_regular_file: ca_target_metadata
            .as_ref()
            .map(|metadata| metadata.file_type().is_file() && !metadata.file_type().is_symlink()),
        ca_target_mode: ca_target_metadata
            .as_ref()
            .map(|metadata| metadata.permissions().mode() & 0o777),
        ca_target_owner_id: ca_target_metadata.as_ref().map(MetadataExt::uid),
        ca_target_group_id: ca_target_metadata.as_ref().map(MetadataExt::gid),
        ca_digest_matches,
        secret_exposed: false,
        unknowns,
    }
}

fn metadata(path: &Path, label: &str, unknowns: &mut Vec<String>) -> Option<std::fs::Metadata> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            unknowns.push(format!("{label}_metadata_unavailable:{error}"));
            None
        }
    }
}

fn secret_source_admissible(path: &Path, max_bytes: u64) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| {
        metadata.file_type().is_file()
            && !metadata.file_type().is_symlink()
            && metadata.len() > 0
            && metadata.len() <= max_bytes
            && metadata.permissions().mode() & 0o077 == 0
    })
}

fn public_source_admissible(path: &Path, max_bytes: u64) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| {
        metadata.file_type().is_file()
            && !metadata.file_type().is_symlink()
            && metadata.len() > 0
            && metadata.len() <= max_bytes
    })
}

pub(super) fn file_digest(path: &Path) -> Option<String> {
    fs::read(path)
        .ok()
        .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
}
