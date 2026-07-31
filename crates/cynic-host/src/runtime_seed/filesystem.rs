use std::{
    fs::{self, File},
    io::{Read, Write},
    os::unix::fs::symlink,
    path::{Component, Path},
};

use cynic_protocol::RuntimeSeedRequest;
use sha2::{Digest, Sha256};

use crate::update_lifecycle::ArgvRunner;

use super::{
    BUNDLE_MARKER, RELEASE_MARKER, RuntimeSeedBundleMarker, RuntimeSeedReleaseMarker,
    signature::RuntimeSeedSignatureVerifier,
};

pub(super) fn verify_apply_preconditions(
    request: &RuntimeSeedRequest,
    verifier: &dyn RuntimeSeedSignatureVerifier,
) -> Result<(), String> {
    if observed_hostname() != request.node_id {
        return Err("runtime_seed_node_identity_mismatch".to_string());
    }
    if sha256_file(Path::new(&request.basket_path))? != request.basket_digest {
        return Err("runtime_seed_basket_digest_mismatch".to_string());
    }
    verifier.verify(request)
}

pub(super) fn inspect_archive<R: ArgvRunner>(
    request: &RuntimeSeedRequest,
    runner: &R,
) -> Result<(), String> {
    let output = runner.run(&["tar", "-tf", &request.basket_path]);
    if !output.success() {
        return Err(format!(
            "runtime_seed_archive_inspection_failed:{}",
            output.stderr.trim()
        ));
    }
    if output
        .stdout
        .lines()
        .any(|entry| !safe_archive_entry(entry))
    {
        return Err("runtime_seed_archive_entry_unsafe".to_string());
    }
    let verbose = runner.run(&["tar", "-tvf", &request.basket_path]);
    if !verbose.success()
        || verbose.stdout.lines().any(|entry| {
            !entry
                .as_bytes()
                .first()
                .is_some_and(|byte| matches!(*byte, b'-' | b'd'))
        })
    {
        return Err("runtime_seed_archive_type_unsafe".to_string());
    }
    Ok(())
}

pub(super) fn extract_archive<R: ArgvRunner>(
    request: &RuntimeSeedRequest,
    staging: &Path,
    runner: &R,
) -> Result<(), String> {
    let parent = staging
        .parent()
        .ok_or_else(|| "runtime_seed_staging_parent_missing".to_string())?;
    ensure_safe_directory_chain(Path::new(&request.install_root), parent)?;
    fs::create_dir(staging)
        .map_err(|error| format!("runtime_seed_staging_create_failed:{error}"))?;
    let staging = staging
        .to_str()
        .ok_or_else(|| "runtime_seed_staging_path_not_utf8".to_string())?;
    let output = runner.run(&[
        "tar",
        "--extract",
        "--file",
        &request.basket_path,
        "--directory",
        staging,
        "--no-same-owner",
        "--no-same-permissions",
    ]);
    if output.success() {
        Ok(())
    } else {
        Err(format!(
            "runtime_seed_archive_extract_failed:{}",
            output.stderr.trim()
        ))
    }
}

fn ensure_safe_directory_chain(root: &Path, target: &Path) -> Result<(), String> {
    let relative = target
        .strip_prefix(root)
        .map_err(|_| "runtime_seed_staging_parent_outside_root".to_string())?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err("runtime_seed_staging_parent_unsafe".to_string());
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Err("runtime_seed_staging_parent_unsafe".to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|error| {
                    format!("runtime_seed_staging_parent_create_failed:{error}")
                })?;
            }
            Err(error) => {
                return Err(format!("runtime_seed_staging_parent_unobservable:{error}"));
            }
        }
    }
    Ok(())
}

pub(super) fn verify_bundle_marker(
    request: &RuntimeSeedRequest,
    staging: &Path,
) -> Result<(), String> {
    let marker: RuntimeSeedBundleMarker = serde_json::from_slice(
        &fs::read(staging.join(BUNDLE_MARKER))
            .map_err(|error| format!("runtime_seed_bundle_marker_read_failed:{error}"))?,
    )
    .map_err(|error| format!("runtime_seed_bundle_marker_invalid:{error}"))?;
    if marker.release_id != request.release_id || marker.release_epoch != request.release_epoch {
        return Err("runtime_seed_bundle_marker_mismatch".to_string());
    }
    Ok(())
}

pub(super) fn persist_release_marker(
    request: &RuntimeSeedRequest,
    staging: &Path,
) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(&RuntimeSeedReleaseMarker {
        release_id: request.release_id.clone(),
        release_epoch: request.release_epoch,
        basket_digest: request.basket_digest.clone(),
    })
    .map_err(|error| format!("runtime_seed_release_marker_encode_failed:{error}"))?;
    let path = staging.join(RELEASE_MARKER);
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("runtime_seed_release_marker_create_failed:{error}"))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("runtime_seed_release_marker_write_failed:{error}"))
}

pub(super) fn activate_link(root: &Path, link: &Path, target: &Path) -> Result<(), String> {
    let name = link
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "runtime_seed_link_name_invalid".to_string())?;
    let temporary = root.join(format!(".{name}.next"));
    if fs::symlink_metadata(&temporary).is_ok() {
        return Err("runtime_seed_temporary_link_exists".to_string());
    }
    symlink(target, &temporary)
        .map_err(|error| format!("runtime_seed_link_create_failed:{error}"))?;
    fs::rename(&temporary, link).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("runtime_seed_link_promote_failed:{error}")
    })?;
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("runtime_seed_root_sync_failed:{error}"))
}

pub(super) fn read_link_marker(
    root: &Path,
    relative_link: &str,
) -> Result<RuntimeSeedReleaseMarker, String> {
    let link = root.join(relative_link);
    let metadata = fs::symlink_metadata(&link)
        .map_err(|error| format!("runtime_seed_link_unobservable:{error}"))?;
    if !metadata.file_type().is_symlink() {
        return Err("runtime_seed_link_not_symlink".to_string());
    }
    let target =
        fs::read_link(&link).map_err(|error| format!("runtime_seed_link_read_failed:{error}"))?;
    if target.is_absolute()
        || target
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("runtime_seed_link_target_unsafe".to_string());
    }
    serde_json::from_slice(
        &fs::read(root.join(target).join(RELEASE_MARKER))
            .map_err(|error| format!("runtime_seed_release_marker_read_failed:{error}"))?,
    )
    .map_err(|error| format!("runtime_seed_release_marker_invalid:{error}"))
}

pub(super) fn ensure_root(root: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|error| format!("runtime_seed_install_root_unobservable:{error}"))?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        Ok(())
    } else {
        Err("runtime_seed_install_root_unsafe".to_string())
    }
}

pub(super) fn regular_file(path: &str, field: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("runtime_seed_{field}_unobservable:{error}"))?;
    if metadata.is_file() && !metadata.file_type().is_symlink() {
        Ok(())
    } else {
        Err(format!("runtime_seed_{field}_unsafe"))
    }
}

pub(super) fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("runtime_seed_basket_read_failed:{error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("runtime_seed_basket_read_failed:{error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
}

fn safe_archive_entry(entry: &str) -> bool {
    !entry.is_empty()
        && !Path::new(entry).is_absolute()
        && Path::new(entry)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

pub(super) fn rollback_staging(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

pub(super) fn observed_hostname() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            fs::read_to_string("/etc/hostname")
                .ok()
                .map(|value| value.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}
