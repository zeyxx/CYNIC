//! [`MemFilesystem`]: in-memory [`FilesystemPort`] fake for unit tests.
//!
//! Public so downstream crates can prove their own build-adjacent logic
//! without touching a real disk. Never used in production paths.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::ports::FilesystemPort;

/// In-memory filesystem. Mutex-guarded so the fake satisfies the port's
/// `Send + Sync` bounds; build proofs are computed, never contended.
#[derive(Debug, Default)]
pub struct MemFilesystem {
    files: Mutex<HashMap<PathBuf, Vec<u8>>>,
}

impl MemFilesystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a path, bypassing the port (test setup only).
    pub fn seed(&self, path: &Path, bytes: &[u8]) {
        self.files
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), bytes.to_vec());
    }

    /// Read back a path, bypassing the port (test assertions only).
    pub fn read(&self, path: &Path) -> Option<Vec<u8>> {
        self.files.lock().unwrap().get(path).cloned()
    }
}

impl FilesystemPort for MemFilesystem {
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| format!("mem_read_missing:{}", path.display()))
    }

    fn copy(&self, source: &Path, target: &Path) -> Result<(), String> {
        let bytes = self.read_bytes(source)?;
        self.files
            .lock()
            .unwrap()
            .insert(target.to_path_buf(), bytes);
        Ok(())
    }

    fn write_bytes(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        self.files
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    fn rename(&self, source: &Path, target: &Path) -> Result<(), String> {
        let mut files = self.files.lock().unwrap();
        let bytes = files
            .remove(source)
            .ok_or_else(|| format!("mem_rename_missing:{}", source.display()))?;
        files.insert(target.to_path_buf(), bytes);
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> Result<(), String> {
        self.files.lock().unwrap().remove(path);
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.lock().unwrap().contains_key(path)
    }
}
