//! Command execution abstractions for VCS operations.
//!
//! Layer-pure ports: defines runner traits without concrete OS process bindings.
//! Concrete implementations (LocalVcsRunner) live in `cynic-host`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsCommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub spawn_error: Option<String>,
}

pub trait VcsRunner: Send + Sync {
    fn run(&self, argv: &[&str]) -> VcsCommandOutput;
}

/// String-oriented argv runner and output — the single L0 port, re-exported
/// so existing `crate::runner::` paths keep working. The local copies this
/// replaces had a narrower contract (no `Serialize`, `MapArgvRunner` hidden
/// behind `cfg(test)`); `VcsRunner`/`VcsCommandOutput` above stay separate
/// because git output is byte-oriented and must not round-trip through `String`.
pub use cynic_protocol::argv::{ArgvOutput, ArgvRunner, MapArgvRunner};
