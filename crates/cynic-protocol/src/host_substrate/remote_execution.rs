//! Pure remote execution port — argv construction, zero I/O.
//!
//! Policies name a target and a bounded remote argv; adapters decide whether
//! that target is reached through SSH, an agent bridge, or another transport.
//! The port lives in `cynic-protocol` (L0) so that any L1 adapter
//! (`cynic-proxmox`, `cynic-host`) can consume it without inversion.

/// Portable remote execution command construction.
pub trait RemoteExecutionPort: Send + Sync {
    /// Construct the argv to execute `remote_argv` on `target`.
    fn exec_argv(&self, target: &str, remote_argv: &[&str]) -> Result<Vec<String>, String>;

    /// Construct the argv to copy `source` to `target`.
    fn copy_argv(&self, source: &str, target: &str) -> Result<Vec<String>, String>;
}
