//! Ports for external environment observation (Git, filesystem) in audit rules.

use cynic_protocol::argv::ArgvOutput;

/// Port for observing git repository state without direct process spawning in domain core.
pub trait GitObserverPort: Send + Sync {
    /// Execute a git command with the given argument list against the repository.
    fn run_git(&self, args: &[&str]) -> ArgvOutput;

    /// Compute the stable git patch-id for a given commit hash.
    fn patch_id(&self, commit: &str) -> Option<String>;
}
