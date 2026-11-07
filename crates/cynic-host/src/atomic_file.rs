//! Adapter OS — shared atomic file replacement for cynic-host.
//!
//! Single implementation of the "write to a staging file, `sync_all`,
//! `rename` over the target" pattern previously duplicated in
//! `agent_access::authority_bundle`, `framework_bootstrap::machine_enrollment`
//! and `framework_bootstrap::runtime_binding`. The staging file is created
//! with `create_new` (never truncates an existing staging file) and removed
//! on any failure, so a failed write never leaves a partial target behind.
//!
//! Semantics per call site are preserved through the options:
//! - `mode`: when `Some`, the staging file is created `0o600` and then
//!   `chmod`ed to `mode` before the rename (bootstrap authorities);
//!   `None` keeps the process-default creation mode (authority bundle).
//! - `owner_group`: when `Some`, the staging file is `chown`ed to it
//!   before the rename.
//! - `create_parent`: create the parent directory when missing
//!   (runtime-binding environment file); callers that validate the parent
//!   themselves pass `false`.
//!
//! Errors are reported as `{error_prefix}: {io error}` so each call site
//! keeps its own diagnostic context.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{chown, OpenOptionsExt, PermissionsExt};
use std::path::Path;

pub(crate) fn write_atomic(
    path: &Path,
    bytes: &[u8],
    mode: Option<u32>,
    owner_group: Option<(u32, u32)>,
    create_parent: bool,
    error_prefix: &str,
) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if create_parent {
        fs::create_dir_all(parent)
            .map_err(|error| format!("{error_prefix}_parent_failed:{error}"))?;
    }
    let temporary = temporary_path(path);
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        if mode.is_some() {
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|error| format!("{error_prefix}: {error}"))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("{error_prefix}: {error}"))?;
        if let Some(mode) = mode {
            file.set_permissions(fs::Permissions::from_mode(mode))
                .map_err(|error| format!("{error_prefix}: {error}"))?;
        }
        if let Some((owner_id, group_id)) = owner_group {
            chown(&temporary, Some(owner_id), Some(group_id))
                .map_err(|error| format!("{error_prefix}: {error}"))?;
        }
        fs::rename(&temporary, path).map_err(|error| format!("{error_prefix}: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn temporary_path(path: &Path) -> std::path::PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("atomic-write.tmp");
    path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomic_replaces_the_target_and_leaves_no_staging_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("target.txt");
        fs::write(&path, "old").unwrap();

        write_atomic(&path, b"new", None, None, false, "test_write").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        let leftovers: Vec<_> = fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name.to_string_lossy().starts_with('.'))
            .collect();
        assert!(leftovers.is_empty(), "staging leftovers: {leftovers:?}");
    }

    #[test]
    fn write_atomic_applies_mode_and_ownership_before_rename() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bound.txt");
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };

        write_atomic(&path, b"bound", Some(0o640), Some((uid, gid)), false, "test_mode")
            .unwrap();

        use std::os::unix::fs::MetadataExt;
        let metadata = fs::metadata(&path).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o640);
        assert_eq!(fs::read_to_string(&path).unwrap(), "bound");
    }

    #[test]
    fn write_atomic_reports_a_missing_parent_instead_of_panicking() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("absent").join("target.txt");

        let error = write_atomic(&path, b"x", None, None, false, "test_missing").unwrap_err();
        assert!(error.starts_with("test_missing:"), "error: {error}");
    }
}
