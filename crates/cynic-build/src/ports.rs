//! Driven ports of the build bounded context (hexagonal architecture).
//!
//! The domain (types.rs, runner.rs) depends only on these traits. Adapters
//! in adapters/ provide the OS-backed implementations; tests and downstream
//! crates provide fakes. No type in this file performs I/O.

use std::path::Path;

/// Port for executing build commands (dependency inversion).
pub trait BuildRunner: Send + Sync {
    fn run_command(&self, argv: &[&str]) -> BuildCommandOutput;
}

/// Value object returned through the [`BuildRunner`] port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub spawn_error: Option<String>,
}

impl BuildCommandOutput {
    pub fn success(&self) -> bool {
        self.spawn_error.is_none() && self.exit_code == Some(0)
    }

    pub fn summary(&self) -> String {
        if let Some(err) = &self.spawn_error {
            return format!("spawn error: {err}");
        }
        let code = self
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "none".into());
        let msg = if !self.stdout.trim().is_empty() {
            self.stdout.trim()
        } else if !self.stderr.trim().is_empty() {
            self.stderr.trim()
        } else {
            "empty output"
        };
        format!("exit {code}: {msg}")
    }
}

/// Port for the filesystem effects of the build pipeline: artifact
/// read-back and atomic installation with rollback.
///
/// Errors are opaque strings so adapters never leak OS error types into
/// the domain. Paths stay as [`Path`] so the domain never formats them.
pub trait FilesystemPort: Send + Sync {
    /// Readiness error type is folded into strings at the adapter edge.
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String>;
    fn copy(&self, source: &Path, target: &Path) -> Result<(), String>;
    fn write_bytes(&self, path: &Path, bytes: &[u8]) -> Result<(), String>;
    fn rename(&self, source: &Path, target: &Path) -> Result<(), String>;
    fn remove_file(&self, path: &Path) -> Result<(), String>;
    fn exists(&self, path: &Path) -> bool;
}

/// Port for process-identity observations the pipeline must not shell out
/// to obtain (no `id -u` subprocess, no ambient uid reads in the domain).
pub trait IdentityPort: Send + Sync {
    fn uid(&self) -> u32;
    fn pid(&self) -> u32;
}
