//! [`StdFilesystem`]: the production [`FilesystemPort`] adapter.
//!
//! Thin, total mapping from port operations to `std::fs`. No policy, no
//! retries, no path canonicalization: any decision stays in runner.rs.

use std::path::Path;

use crate::ports::FilesystemPort;

/// OS filesystem adapter. Construct once at the composition root and pass
/// by reference; it carries no state.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdFilesystem;

impl FilesystemPort for StdFilesystem {
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|error| format!("fs_read_failed:{error}"))
    }

    fn copy(&self, source: &Path, target: &Path) -> Result<(), String> {
        std::fs::copy(source, target)
            .map(|_| ())
            .map_err(|error| format!("fs_copy_failed:{error}"))
    }

    fn write_bytes(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        std::fs::write(path, bytes).map_err(|error| format!("fs_write_failed:{error}"))
    }

    fn rename(&self, source: &Path, target: &Path) -> Result<(), String> {
        std::fs::rename(source, target).map_err(|error| format!("fs_rename_failed:{error}"))
    }

    fn remove_file(&self, path: &Path) -> Result<(), String> {
        std::fs::remove_file(path).map_err(|error| format!("fs_remove_failed:{error}"))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}
