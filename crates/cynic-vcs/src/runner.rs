//! Command execution abstractions for VCS operations.
//!
//! Layer-pure ports: defines runner traits without concrete OS process bindings.
//! Concrete local implementation `LocalVcsRunner` lives here.

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
pub use cynic_protocol::argv::{ArgvOutput, ArgvRunner};


/// Local process-backed VCS runner.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalVcsRunner;

impl VcsRunner for LocalVcsRunner {
    fn run(&self, argv: &[&str]) -> VcsCommandOutput {
        let output = cynic_process::LocalArgvRunner.run(argv);
        VcsCommandOutput {
            exit_code: output.exit_code,
            stdout: output.stdout.into_bytes(),
            stderr: output.stderr.into_bytes(),
            spawn_error: output.spawn_error,
        }
    }
}

impl ArgvRunner for LocalVcsRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        cynic_process::LocalArgvRunner.run(argv)
    }
}

/// Test fake from the owning crate. `cynic-vcs` (L2) may depend on
/// `cynic-lifecycle` (L1) in tests — downward dependency, DAG-legal — but
/// production code must never fabricate runners.
#[cfg(test)]
pub use cynic_process::MapArgvRunner;
