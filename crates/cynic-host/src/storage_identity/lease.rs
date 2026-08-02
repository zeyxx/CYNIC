use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct StorageIdentityEnrollmentLease {
    _file: File,
}

impl StorageIdentityEnrollmentLease {
    pub(crate) fn acquire() -> Result<Self, String> {
        if unsafe { libc::geteuid() } != 0 {
            return Err(
                "storage identity enrollment requires the admitted root helper".to_string(),
            );
        }
        let path = lock_path();
        let parent = path
            .parent()
            .ok_or_else(|| "storage identity lock has no parent".to_string())?;
        if !parent.is_dir() {
            return Err(format!(
                "storage identity authority directory is absent: {}",
                parent.display()
            ));
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|error| format!("storage identity lock unavailable: {error}"))?;
        let metadata = file
            .metadata()
            .map_err(|error| format!("storage identity lock metadata unavailable: {error}"))?;
        if !metadata.file_type().is_file() || metadata.uid() != 0 {
            return Err("storage identity lock is not a root-owned regular file".to_string());
        }
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("storage identity lock mode repair failed: {error}"))?;
        let status = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if status != 0 {
            let error = std::io::Error::last_os_error();
            return Err(format!(
                "storage identity enrollment is already active or cannot be fenced: {error}"
            ));
        }
        Ok(Self { _file: file })
    }
}

fn lock_path() -> PathBuf {
    crate::runtime_authority::default_runtime_authority_paths()
        .config_root
        .join(".storage-identity-enroll.lock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_is_derived_from_runtime_authority() {
        assert_eq!(
            lock_path(),
            PathBuf::from("/etc/cynic/.storage-identity-enroll.lock")
        );
    }
}
