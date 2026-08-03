use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::Serialize;
use zeroize::Zeroize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSecretInstallSpec {
    pub identity_id: String,
    pub target: PathBuf,
    pub owner_id: u32,
    pub group_id: Option<u32>,
    pub mode: u32,
}

impl RuntimeSecretInstallSpec {
    pub fn storage_runtime() -> Self {
        let authority = crate::runtime_authority::default_runtime_authority_paths();
        let group = authority.principal.runtime_group;
        Self {
            identity_id: "storage-runtime".to_string(),
            target: authority
                .config_root
                .join("secrets/storage-runtime-password"),
            owner_id: 0,
            group_id: crate::identity_database::resolve_identity_id("/etc/group", &group),
            mode: 0o640,
        }
    }

    pub fn provider_control(provider: &str) -> Result<Self, String> {
        if provider != "proxmox" {
            return Err("provider runtime secret adapter is unavailable".to_string());
        }
        let authority = crate::runtime_authority::default_runtime_authority_paths();
        let group = authority.principal.runtime_group;
        Ok(Self {
            identity_id: "provider-control:proxmox".to_string(),
            target: authority.config_root.join("secrets/proxmox-token"),
            owner_id: 0,
            group_id: crate::identity_database::resolve_identity_id("/etc/group", &group),
            mode: 0o640,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeSecretObservation {
    pub identity_id: String,
    pub path: String,
    pub present: bool,
    pub regular_file: Option<bool>,
    pub symlink: Option<bool>,
    pub owner_id: Option<u32>,
    pub group_id: Option<u32>,
    pub mode: Option<u32>,
    pub non_empty: Option<bool>,
    pub secret_exposed: bool,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeSecretInstallReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub identity_id: String,
    pub mutated: bool,
    pub before: RuntimeSecretObservation,
    pub after: RuntimeSecretObservation,
    pub required_secret_class: &'static str,
    pub secret_exposed: bool,
    pub proofs: Vec<String>,
    pub rollback_intent: String,
    pub unknowns: Vec<String>,
}

pub(crate) fn observe_runtime_secret(spec: &RuntimeSecretInstallSpec) -> RuntimeSecretObservation {
    let mut unknowns = Vec::new();
    let metadata = match fs::symlink_metadata(&spec.target) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            unknowns.push(format!("runtime_secret_metadata_unavailable:{error}"));
            None
        }
    };
    RuntimeSecretObservation {
        identity_id: spec.identity_id.clone(),
        path: spec.target.display().to_string(),
        present: metadata.is_some(),
        regular_file: metadata.as_ref().map(|value| value.file_type().is_file()),
        symlink: metadata
            .as_ref()
            .map(|value| value.file_type().is_symlink()),
        owner_id: metadata.as_ref().map(MetadataExt::uid),
        group_id: metadata.as_ref().map(MetadataExt::gid),
        mode: metadata
            .as_ref()
            .map(|value| value.permissions().mode() & 0o777),
        non_empty: metadata.as_ref().map(|value| value.len() > 0),
        secret_exposed: false,
        unknowns,
    }
}

pub(crate) fn install_runtime_secret(
    spec: &RuntimeSecretInstallSpec,
    material: &crate::secret_material::SecretMaterial,
) -> Result<RuntimeSecretInstallReport, String> {
    validate_spec(spec)?;
    if unsafe { libc::geteuid() } != 0 {
        return Err("runtime secret installation requires the admitted root helper".to_string());
    }
    let before = observe_runtime_secret(spec);
    if before.present {
        return Err(
            "runtime identity already exists; isolated file rotation is forbidden".to_string(),
        );
    }

    install_protected_runtime_file(
        &spec.identity_id,
        &spec.target,
        spec.owner_id,
        spec.group_id.expect("validated group"),
        spec.mode,
        material.expose_bytes(),
    )?;
    let after = observe_runtime_secret(spec);
    let mut unknowns = after.unknowns.clone();
    let converged = after.present
        && after.regular_file == Some(true)
        && after.symlink == Some(false)
        && after.owner_id == Some(spec.owner_id)
        && after.group_id == spec.group_id
        && after.mode == Some(spec.mode)
        && after.non_empty == Some(true);
    if !converged {
        unknowns.push("runtime_secret_read_back_not_converged".to_string());
    }

    Ok(RuntimeSecretInstallReport {
        schema: "cynic.runtime_secret_install_report.v1",
        status: if converged { "completed" } else { "blocked" },
        identity_id: spec.identity_id.clone(),
        mutated: true,
        before,
        after,
        required_secret_class: "RuntimeMachineIdentity",
        secret_exposed: false,
        proofs: if converged {
            vec![
                "credential_presence_without_value".to_string(),
                "owner_group_mode_read_back".to_string(),
                "regular_non_symlink_file_read_back".to_string(),
            ]
        } else {
            Vec::new()
        },
        rollback_intent: format!(
            "remove only the newly installed {} credential if the enclosing enrollment transaction fails; rotation requires a separate atomic authority-and-file contract",
            spec.identity_id
        ),
        unknowns,
    })
}

pub(crate) fn remove_runtime_secret_if_matches(
    spec: &RuntimeSecretInstallSpec,
    material: &crate::secret_material::SecretMaterial,
) -> Result<RuntimeSecretObservation, String> {
    validate_spec(spec)?;
    if unsafe { libc::geteuid() } != 0 {
        return Err("runtime secret compensation requires the admitted root helper".to_string());
    }
    remove_protected_runtime_file_if_matches(&spec.target, material.expose_bytes())?;
    Ok(observe_runtime_secret(spec))
}

fn validate_spec(spec: &RuntimeSecretInstallSpec) -> Result<(), String> {
    if spec.owner_id != 0 || spec.group_id.is_none() || spec.mode != 0o640 {
        return Err("runtime secret install spec is not admitted".to_string());
    }
    let canonical = match spec.identity_id.as_str() {
        "storage-runtime" => RuntimeSecretInstallSpec::storage_runtime(),
        "provider-control:proxmox" => RuntimeSecretInstallSpec::provider_control("proxmox")?,
        _ => return Err("runtime secret identity is not admitted".to_string()),
    };
    if spec.target != canonical.target || spec.group_id != canonical.group_id {
        return Err(
            "runtime secret target or principal differs from runtime authority".to_string(),
        );
    }
    Ok(())
}

pub(crate) fn install_protected_runtime_file(
    identity_id: &str,
    target: &Path,
    owner_id: u32,
    group_id: u32,
    mode: u32,
    material: &[u8],
) -> Result<(), String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("protected runtime file installation requires root authority".to_string());
    }
    if material.is_empty() || material.len() > 64 * 1024 {
        return Err("protected runtime file material is empty or exceeds the bound".to_string());
    }
    let authority = crate::runtime_authority::default_runtime_authority_paths();
    if !target.starts_with(&authority.config_root)
        || target
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("protected runtime file target is outside runtime authority".to_string());
    }
    let parent = target
        .parent()
        .ok_or_else(|| "runtime secret target has no parent".to_string())?;
    let directory_spec = RuntimeSecretInstallSpec {
        identity_id: identity_id.to_string(),
        target: target.to_path_buf(),
        owner_id,
        group_id: Some(group_id),
        mode,
    };
    prepare_secret_directory(&directory_spec, parent)?;

    let temporary = temporary_path(parent, identity_id);
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| format!("runtime secret staging failed: {error}"))?;
    let result = install_open_file(&mut file, &directory_spec, material)
        .and_then(|()| rename_noreplace(&temporary, target));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("runtime secret atomic install failed: {error}"))?;
    sync_directory(parent)
}

pub(crate) fn remove_protected_runtime_file_if_matches(
    target: &Path,
    material: &[u8],
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(target)
        .map_err(|error| format!("protected runtime compensation open failed: {error}"))?;
    let metadata_before = file
        .metadata()
        .map_err(|error| format!("protected runtime compensation metadata failed: {error}"))?;
    if !metadata_before.file_type().is_file() || metadata_before.len() > 64 * 1024 {
        return Err("protected runtime compensation target is not a bounded file".to_string());
    }
    let mut installed = Vec::with_capacity(metadata_before.len() as usize);
    file.read_to_end(&mut installed)
        .map_err(|error| format!("protected runtime compensation read failed: {error}"))?;
    let matches = installed == material;
    installed.zeroize();
    if !matches {
        return Err("protected runtime compensation refused non-matching material".to_string());
    }
    let metadata_after = fs::symlink_metadata(target)
        .map_err(|error| format!("protected runtime compensation revalidation failed: {error}"))?;
    if !metadata_after.file_type().is_file()
        || metadata_before.dev() != metadata_after.dev()
        || metadata_before.ino() != metadata_after.ino()
    {
        return Err("protected runtime file changed during compensation".to_string());
    }
    fs::remove_file(target)
        .map_err(|error| format!("protected runtime compensation removal failed: {error}"))?;
    if let Some(parent) = target.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}

fn prepare_secret_directory(spec: &RuntimeSecretInstallSpec, parent: &Path) -> Result<(), String> {
    let authority = crate::runtime_authority::default_runtime_authority_paths();
    let root_metadata = fs::symlink_metadata(&authority.config_root)
        .map_err(|error| format!("runtime config authority is unavailable: {error}"))?;
    if !root_metadata.file_type().is_dir()
        || root_metadata.file_type().is_symlink()
        || root_metadata.uid() != 0
        || root_metadata.permissions().mode() & 0o022 != 0
    {
        return Err("runtime config authority is not a root-owned protected directory".to_string());
    }
    match fs::symlink_metadata(parent) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(parent)
            .map_err(|error| format!("runtime secret directory creation failed: {error}"))?,
        Err(error) => {
            return Err(format!(
                "runtime secret directory observation failed: {error}"
            ));
        }
    }
    let metadata = fs::symlink_metadata(parent)
        .map_err(|error| format!("runtime secret directory metadata failed: {error}"))?;
    if !metadata.file_type().is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != spec.owner_id
        || metadata.permissions().mode() & 0o022 != 0
    {
        return Err("runtime secret directory is not a protected authority directory".to_string());
    }
    chown_path(
        parent,
        spec.owner_id,
        spec.group_id.expect("validated group"),
    )?;
    fs::set_permissions(parent, fs::Permissions::from_mode(0o750))
        .map_err(|error| format!("runtime secret directory mode failed: {error}"))
}

fn rename_noreplace(source: &Path, target: &Path) -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    let source = std::ffi::CString::new(source.as_os_str().as_bytes())
        .map_err(|_| "runtime secret staging path contains a null byte".to_string())?;
    let target = std::ffi::CString::new(target.as_os_str().as_bytes())
        .map_err(|_| "runtime secret target path contains a null byte".to_string())?;
    let status = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if status != 0 {
        return Err(format!(
            "no-replace activation failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

fn install_open_file(
    file: &mut File,
    spec: &RuntimeSecretInstallSpec,
    material: &[u8],
) -> Result<(), String> {
    file.write_all(material)
        .map_err(|error| format!("write failed: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("staging sync failed: {error}"))?;
    let status = unsafe {
        libc::fchown(
            file.as_raw_fd(),
            spec.owner_id,
            spec.group_id.expect("validated group"),
        )
    };
    if status != 0 {
        return Err(format!(
            "staging ownership failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    file.set_permissions(fs::Permissions::from_mode(spec.mode))
        .map_err(|error| format!("staging mode failed: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("staging metadata sync failed: {error}"))
}

fn temporary_path(parent: &Path, identity_id: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    parent.join(format!(
        ".{identity_id}.{}.{}.tmp",
        std::process::id(),
        nonce
    ))
}

fn chown_path(path: &Path, owner: u32, group: u32) -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| "runtime secret path contains a null byte".to_string())?;
    if unsafe { libc::chown(path.as_ptr(), owner, group) } != 0 {
        return Err(format!(
            "runtime secret directory ownership failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("runtime secret directory sync failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_never_contains_secret_material() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("runtime-secret");
        fs::write(&target, "high-entropy-runtime-value").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
        let metadata = fs::metadata(&target).unwrap();
        let spec = RuntimeSecretInstallSpec {
            identity_id: "storage-runtime".to_string(),
            target,
            owner_id: metadata.uid(),
            group_id: Some(metadata.gid()),
            mode: 0o640,
        };

        let json = serde_json::to_string(&observe_runtime_secret(&spec)).unwrap();
        assert!(!json.contains("high-entropy-runtime-value"));
        assert!(json.contains("secret_exposed\":false"));
    }

    #[test]
    fn arbitrary_secret_target_is_not_admitted() {
        let canonical = RuntimeSecretInstallSpec::storage_runtime();
        let mut tampered = canonical.clone();
        tampered.target = PathBuf::from("/tmp/not-runtime-authority");

        let error = validate_spec(&tampered).unwrap_err();
        assert!(error.contains("differs from runtime authority"));
    }
}
