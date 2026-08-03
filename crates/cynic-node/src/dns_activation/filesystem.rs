use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use sha2::Digest;

use super::types::DnsActivationSpec;

pub(super) struct Backup {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
    mode: Option<u32>,
}

pub(super) fn capture_backups(spec: &DnsActivationSpec) -> Result<Vec<Backup>, String> {
    spec.files
        .iter()
        .map(|file| {
            let path = PathBuf::from(&file.target);
            match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                    Ok(Backup {
                        path: path.clone(),
                        bytes: Some(fs::read(&path).map_err(|error| {
                            format!("dns_activation_backup_read_failed:{error}")
                        })?),
                        mode: Some(metadata.permissions().mode() & 0o7777),
                    })
                }
                Ok(_) => Err(format!("dns_activation_target_unsafe:{}", path.display())),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Backup {
                    path,
                    bytes: None,
                    mode: None,
                }),
                Err(error) => Err(format!("dns_activation_target_unobservable:{error}")),
            }
        })
        .collect()
}

pub(super) fn rollback_files_and_directories(
    backups: &[Backup],
    created_directories: &[PathBuf],
) -> Result<(), String> {
    let mut errors = Vec::new();
    if let Err(error) = restore_backups(backups) {
        errors.push(error);
    }
    for directory in created_directories.iter().rev() {
        if let Err(error) = fs::remove_dir(directory) {
            if error.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!(
                    "dns_activation_rollback_directory_remove_failed:{}:{error}",
                    directory.display()
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(";"))
    }
}

pub(super) fn missing_parent_directories(spec: &DnsActivationSpec) -> Result<Vec<PathBuf>, String> {
    let mut missing = BTreeSet::new();
    for file in &spec.files {
        let parent = Path::new(&file.target)
            .parent()
            .ok_or_else(|| "dns_activation_target_parent_missing".to_string())?;
        validate_parent_chain(parent, &mut missing)?;
    }
    let mut paths = missing.into_iter().collect::<Vec<_>>();
    paths.sort_by_key(|path| path.components().count());
    Ok(paths)
}

pub(super) fn atomic_install(
    source: &Path,
    target: &Path,
    mode: u32,
    expected_sha256: &str,
) -> Result<(), String> {
    let bytes =
        fs::read(source).map_err(|error| format!("dns_activation_source_read_failed:{error}"))?;
    let observed_sha256 = format!("sha256:{}", hex::encode(sha2::Sha256::digest(&bytes)));
    if observed_sha256 != expected_sha256 {
        return Err(format!(
            "dns_activation_source_changed_after_plan:{}",
            source.display()
        ));
    }
    atomic_write(target, &bytes, mode)
}

fn restore_backups(backups: &[Backup]) -> Result<(), String> {
    for backup in backups.iter().rev() {
        match &backup.bytes {
            Some(bytes) => atomic_write(&backup.path, bytes, backup.mode.unwrap_or(0o644))?,
            None if backup.path.exists() => fs::remove_file(&backup.path)
                .map_err(|error| format!("dns_activation_rollback_remove_failed:{error}"))?,
            None => {}
        }
    }
    Ok(())
}

fn atomic_write(target: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| "dns_activation_target_parent_missing".to_string())?;
    ensure_directory_chain(parent)?;
    if fs::symlink_metadata(target).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(format!(
            "dns_activation_target_symlink_forbidden:{}",
            target.display()
        ));
    }
    let temporary = parent.join(format!(".cynic-dns-next-{}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        file.set_permissions(fs::Permissions::from_mode(mode))?;
        std::os::unix::fs::chown(&temporary, Some(0), Some(0))?;
        fs::rename(&temporary, target)?;
        fs::File::open(parent)?.sync_all()?;
        Ok::<(), std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("dns_activation_atomic_write_failed:{error}"))
}

fn validate_parent_chain(path: &Path, missing: &mut BTreeSet<PathBuf>) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("dns_activation_target_parent_not_absolute".to_string());
    }
    walk_parent_chain(path, |current, metadata| match metadata {
        Some(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Some(_) => Err(format!(
            "dns_activation_parent_unsafe:{}",
            current.display()
        )),
        None => {
            missing.insert(current.to_path_buf());
            Ok(())
        }
    })
}

fn ensure_directory_chain(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("dns_activation_target_parent_not_absolute".to_string());
    }
    walk_parent_chain(path, |current, metadata| match metadata {
        Some(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Some(_) => Err(format!(
            "dns_activation_parent_unsafe:{}",
            current.display()
        )),
        None => {
            fs::create_dir(current)
                .map_err(|error| format!("dns_activation_parent_create_failed:{error}"))?;
            fs::set_permissions(current, fs::Permissions::from_mode(0o755))
                .map_err(|error| format!("dns_activation_parent_mode_failed:{error}"))?;
            std::os::unix::fs::chown(current, Some(0), Some(0))
                .map_err(|error| format!("dns_activation_parent_owner_failed:{error}"))
        }
    })
}

fn walk_parent_chain(
    path: &Path,
    mut visit: impl FnMut(&Path, Option<fs::Metadata>) -> Result<(), String>,
) -> Result<(), String> {
    let mut current = PathBuf::from("/");
    for component in path.components() {
        let name = match component {
            std::path::Component::RootDir => continue,
            std::path::Component::Normal(name) => name,
            _ => return Err("dns_activation_target_parent_unsafe".to_string()),
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => visit(&current, Some(metadata))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                visit(&current, None)?;
            }
            Err(error) => return Err(format!("dns_activation_parent_unobservable:{error}")),
        }
    }
    Ok(())
}
