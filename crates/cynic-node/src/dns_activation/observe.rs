use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use cynic_lifecycle::ArgvRunner;
use sha2::{Digest, Sha256};

use super::types::{DnsActivationFileObservation, DnsActivationObservation, DnsActivationSpec};

pub(crate) fn observe_dns_activation(
    spec: &DnsActivationSpec,
    runner: &dyn ArgvRunner,
) -> DnsActivationObservation {
    let files = spec
        .files
        .iter()
        .map(|file| observe_file(&file.target))
        .collect::<Vec<_>>();
    let service = runner.run(&[&spec.service_manager_bin, "is-active", &spec.service_unit]);
    let enabled = runner.run(&[&spec.service_manager_bin, "is-enabled", &spec.service_unit]);
    let resolver = runner.run(&[&spec.resolver_bin, "hosts", &spec.probe_fqdn]);
    let mut unknowns = Vec::new();
    if service.spawn_error.is_some() {
        unknowns.push("dns_activation_service_state_unobservable".to_string());
    }
    if enabled.spawn_error.is_some() {
        unknowns.push("dns_activation_service_enablement_unobservable".to_string());
    }
    if resolver.spawn_error.is_some() {
        unknowns.push("dns_activation_resolution_unobservable".to_string());
    }
    let mut evidence_refs = files
        .iter()
        .filter_map(|file| file.sha256.clone())
        .collect::<Vec<_>>();
    evidence_refs.sort();
    evidence_refs.dedup();
    DnsActivationObservation {
        schema: "cynic.dns_activation.observation.v1",
        files,
        service_active: service.spawn_error.is_none().then(|| service.success()),
        service_enabled: enabled.spawn_error.is_none().then(|| enabled.success()),
        probe_resolves: resolver.spawn_error.is_none().then(|| resolver.success()),
        evidence_refs,
        unknowns,
    }
}

fn observe_file(path: &str) -> DnsActivationFileObservation {
    let target = Path::new(path);
    match fs::symlink_metadata(target) {
        Ok(metadata) => {
            let regular = metadata.is_file() && !metadata.file_type().is_symlink();
            let sha256 = regular
                .then(|| fs::read(target).ok())
                .flatten()
                .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))));
            DnsActivationFileObservation {
                target: path.to_string(),
                present: true,
                regular,
                sha256,
                mode: Some(metadata.permissions().mode() & 0o7777),
                owner_uid: Some(metadata.uid()),
                owner_gid: Some(metadata.gid()),
            }
        }
        Err(_) => DnsActivationFileObservation {
            target: path.to_string(),
            present: false,
            regular: false,
            sha256: None,
            mode: None,
            owner_uid: None,
            owner_gid: None,
        },
    }
}
