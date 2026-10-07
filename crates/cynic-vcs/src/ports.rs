//! Command execution abstractions for VCS operations.
//!
//! Layer-pure ports: defines runner traits without concrete OS process bindings.
//! Concrete implementations (LocalVcsRunner) live in `cynic-host`.
//!
//! `ArgvRunner`/`ArgvOutput` were declared here as a third copy of a port that
//! `cynic-lifecycle` already owned, with a narrower contract (no `Serialize`,
//! and its `MapArgvRunner` behind `cfg(test)` so no VCS code could fake a runner
//! in production). Both now come from [`cynic_protocol::argv`].
//!
//! `VcsRunner`/`VcsCommandOutput` are deliberately NOT merged with it: git output
//! is byte-oriented and must stay so, because a diff, a packfile hash and a
//! `--porcelain` stream cannot round-trip through a lossy `String` without
//! changing bytes. Two ports, two payload shapes, one name each.

pub use cynic_protocol::argv::{ArgvOutput, ArgvRunner};

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

/// Observation port for Forgejo. Given a desired state, produce an observed state.
///
/// The concrete observer (`RuntimeForgejoObserver`) lives in `cynic-host`; the
/// domain owns only this boundary.
pub trait ForgejoObservationPort: Send + Sync {
    fn observe(
        &self,
        desired: &crate::types::ForgejoDesiredState,
    ) -> crate::types::ForgejoObservedState;
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct MapArgvRunner {
    pub responses: Vec<(Vec<String>, ArgvOutput)>,
}

#[cfg(test)]
impl ArgvRunner for MapArgvRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        for (pattern, resp) in &self.responses {
            if pattern.iter().map(|s| s.as_str()).eq(argv.iter().copied()) {
                return resp.clone();
            }
        }
        ArgvOutput {
            exit_code: Some(1),
            stdout: String::new(),
            stderr: format!("unmatched argv: {:?}", argv),
            spawn_error: None,
        }
    }
}
