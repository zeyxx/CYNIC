use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use cynic_protocol::RuntimeSeedRequest;
use sha2::{Digest, Sha256};

pub(super) const BUNDLE_REGISTRY_FILE: &str = "registry.json";

pub(super) struct RegistryRollback {
    target: PathBuf,
    backup: PathBuf,
    previous_present: bool,
}

pub(super) fn observe_installed_digest(
    request: &RuntimeSeedRequest,
) -> Result<Option<String>, String> {
    let path = Path::new(&request.registry_path);
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            digest_file(path).map(Some)
        }
        Ok(_) => Err("runtime_seed_registry_target_unsafe".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("runtime_seed_registry_unobservable:{error}")),
    }
}

pub(super) fn verify_staged_registry(
    request: &RuntimeSeedRequest,
    staging: &Path,
) -> Result<(), String> {
    let path = staging.join(BUNDLE_REGISTRY_FILE);
    if digest_file(&path)? == request.registry_digest {
        Ok(())
    } else {
        Err("runtime_seed_registry_digest_mismatch".to_string())
    }
}

pub(super) fn install_registry(
    request: &RuntimeSeedRequest,
    staging: &Path,
) -> Result<RegistryRollback, String> {
    verify_staged_registry(request, staging)?;
    let target = PathBuf::from(&request.registry_path);
    let parent = safe_parent(&target)?;
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "runtime_seed_registry_filename_invalid".to_string())?;
    let next = parent.join(format!(".{file_name}.cynic-next"));
    let backup = parent.join(format!(".{file_name}.cynic-previous"));
    let backup_next = parent.join(format!(".{file_name}.cynic-previous-next"));
    if fs::symlink_metadata(&next).is_ok() {
        return Err("runtime_seed_registry_temporary_exists".to_string());
    }
    let previous_present = match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            if fs::symlink_metadata(&backup_next).is_ok() {
                return Err("runtime_seed_registry_backup_temporary_exists".to_string());
            }
            copy_synced(&target, &backup_next, true)?;
            fs::rename(&backup_next, &backup)
                .map_err(|error| format!("runtime_seed_registry_backup_activate_failed:{error}"))?;
            sync_directory(&parent)?;
            true
        }
        Ok(_) => return Err("runtime_seed_registry_target_unsafe".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(format!("runtime_seed_registry_unobservable:{error}")),
    };
    copy_synced(&staging.join(BUNDLE_REGISTRY_FILE), &next, true)?;
    fs::rename(&next, &target)
        .map_err(|error| format!("runtime_seed_registry_activate_failed:{error}"))?;
    sync_directory(&parent)?;
    let rollback = RegistryRollback {
        target,
        backup,
        previous_present,
    };
    let read_back_error = match observe_installed_digest(request) {
        Ok(Some(digest)) if digest == request.registry_digest => None,
        Ok(_) => Some("runtime_seed_registry_read_back_mismatch".to_string()),
        Err(error) => Some(error),
    };
    if let Some(read_back_error) = read_back_error {
        rollback_registry(&rollback)
            .map_err(|rollback_error| format!("{read_back_error};{rollback_error}"))?;
        return Err(read_back_error);
    }
    Ok(rollback)
}

pub(super) fn install_current_registry(
    request: &RuntimeSeedRequest,
    install_root: &Path,
) -> Result<RegistryRollback, String> {
    let target = fs::read_link(install_root.join(&request.current_link))
        .map_err(|error| format!("runtime_seed_current_link_read_failed:{error}"))?;
    if target.is_absolute()
        || target
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err("runtime_seed_current_link_target_unsafe".to_string());
    }
    install_registry(request, &install_root.join(target))
}

pub(super) fn rollback_registry(rollback: &RegistryRollback) -> Result<(), String> {
    if rollback.previous_present {
        let parent = safe_parent(&rollback.target)?;
        let file_name = rollback
            .target
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "runtime_seed_registry_filename_invalid".to_string())?;
        let next = parent.join(format!(".{file_name}.cynic-rollback"));
        if fs::symlink_metadata(&next).is_ok() {
            return Err("runtime_seed_registry_rollback_temporary_exists".to_string());
        }
        copy_synced(&rollback.backup, &next, true)?;
        fs::rename(&next, &rollback.target)
            .map_err(|error| format!("runtime_seed_registry_rollback_failed:{error}"))?;
        sync_directory(&parent)
    } else {
        fs::remove_file(&rollback.target)
            .map_err(|error| format!("runtime_seed_registry_rollback_remove_failed:{error}"))?;
        sync_directory(safe_parent(&rollback.target)?)
    }
}

fn safe_parent(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("runtime_seed_registry_path_not_absolute".to_string());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "runtime_seed_registry_parent_missing".to_string())?;
    let metadata = fs::symlink_metadata(parent)
        .map_err(|error| format!("runtime_seed_registry_parent_unobservable:{error}"))?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        Ok(parent.to_path_buf())
    } else {
        Err("runtime_seed_registry_parent_unsafe".to_string())
    }
}

fn copy_synced(source: &Path, destination: &Path, create_new: bool) -> Result<(), String> {
    let mut input = File::open(source)
        .map_err(|error| format!("runtime_seed_registry_source_open_failed:{error}"))?;
    let mut options = OpenOptions::new();
    options.write(true).truncate(true);
    if create_new {
        options.create_new(true);
    } else {
        options.create(true);
    }
    let mut output = options
        .open(destination)
        .map_err(|error| format!("runtime_seed_registry_destination_open_failed:{error}"))?;
    std::io::copy(&mut input, &mut output)
        .map_err(|error| format!("runtime_seed_registry_copy_failed:{error}"))?;
    output
        .set_permissions(fs::Permissions::from_mode(0o644))
        .and_then(|()| output.sync_all())
        .map_err(|error| format!("runtime_seed_registry_sync_failed:{error}"))
}

fn digest_file(path: &Path) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("runtime_seed_registry_read_failed:{error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() == 0 {
        return Err("runtime_seed_registry_source_unsafe".to_string());
    }
    let mut input =
        File::open(path).map_err(|error| format!("runtime_seed_registry_read_failed:{error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| format!("runtime_seed_registry_read_failed:{error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
}

fn sync_directory(path: impl AsRef<Path>) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("runtime_seed_registry_directory_sync_failed:{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        RUNTIME_SEED_REQUEST_SCHEMA, RuntimeArtifactSource, RuntimeArtifactTransportKind,
    };

    fn request(root: &Path, digest: String) -> RuntimeSeedRequest {
        RuntimeSeedRequest {
            schema: RUNTIME_SEED_REQUEST_SCHEMA.to_string(),
            node_id: "node-a".to_string(),
            release_id: "release-a".to_string(),
            release_epoch: 1,
            source_commit: "a".repeat(40),
            build_proof_digest: format!("sha256:{}", "b".repeat(64)),
            basket_digest: format!("sha256:{}", "c".repeat(64)),
            registry_digest: digest,
            basket_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::LocalFile,
                locator: "/source/release.tar".to_string(),
            },
            basket_path: "/inbox/release.tar".to_string(),
            detached_signature_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::LocalFile,
                locator: "/source/release.tar.sig".to_string(),
            },
            detached_signature_path: "/inbox/release.tar.sig".to_string(),
            allowed_signers_path: "/etc/cynic/allowed-signers".to_string(),
            registry_path: root.join("etc/registry.json").display().to_string(),
            signer_identity: "release-authority".to_string(),
            signature_namespace: "cynic-runtime-seed".to_string(),
            required_seed_contract_revision: 4,
            install_root: root.join("runtime").display().to_string(),
            staged_slot: "releases/release-a".to_string(),
            current_link: "current".to_string(),
            previous_link: "previous".to_string(),
            postflight_argv: vec!["/bin/true".to_string()],
        }
    }

    #[test]
    fn install_and_rollback_are_proved_by_digest_read_back() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("etc")).unwrap();
        let staging = root.path().join("staging");
        fs::create_dir(&staging).unwrap();
        let old = b"{\"generation\":1}";
        let new = b"{\"generation\":2}";
        fs::write(root.path().join("etc/registry.json"), old).unwrap();
        fs::write(staging.join(BUNDLE_REGISTRY_FILE), new).unwrap();
        let request = request(
            root.path(),
            digest_file(&staging.join(BUNDLE_REGISTRY_FILE)).unwrap(),
        );

        let rollback = install_registry(&request, &staging).unwrap();
        assert_eq!(
            observe_installed_digest(&request).unwrap().as_deref(),
            Some(request.registry_digest.as_str())
        );

        rollback_registry(&rollback).unwrap();
        assert_eq!(
            fs::read(root.path().join("etc/registry.json")).unwrap(),
            old
        );
    }
}
