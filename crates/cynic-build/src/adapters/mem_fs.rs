//! [`MemFilesystem`]: in-memory [`FilesystemPort`] fake for unit tests.
//!
//! Public so downstream crates can prove their own build-adjacent logic
//! without touching a real disk. Never used in production paths.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ports::FilesystemPort;

/// In-memory filesystem. Single-threaded interior mutability is enough:
/// build proofs are computed, never shared across threads mid-proof.
#[derive(Debug, Default)]
pub struct MemFilesystem {
    files: RefCell<HashMap<PathBuf, Vec<u8>>>,
}

impl MemFilesystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a path, bypassing the port (test setup only).
    pub fn seed(&self, path: &Path, bytes: &[u8]) {
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), bytes.to_vec());
    }

    /// Read back a path, bypassing the port (test assertions only).
    pub fn read(&self, path: &Path) -> Option<Vec<u8>> {
        self.files.borrow().get(path).cloned()
    }
}

impl FilesystemPort for MemFilesystem {
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.files
            .borrow()
            .get(path)
            .cloned()
            .ok_or_else(|| format!("mem_read_missing:{}", path.display()))
    }

    fn copy(&self, source: &Path, target: &Path) -> Result<(), String> {
        let bytes = self.read_bytes(source)?;
        self.files.borrow_mut().insert(target.to_path_buf(), bytes);
        Ok(())
    }

    fn write_bytes(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    fn rename(&self, source: &Path, target: &Path) -> Result<(), String> {
        let mut files = self.files.borrow_mut();
        let bytes = files
            .remove(source)
            .ok_or_else(|| format!("mem_rename_missing:{}", source.display()))?;
        files.insert(target.to_path_buf(), bytes);
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> Result<(), String> {
        self.files.borrow_mut().remove(path);
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.borrow().contains_key(path)
    }
}
