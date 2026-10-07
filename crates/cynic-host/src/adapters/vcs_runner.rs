//! Host-level implementation of VCS runners.
//!
//! cynic-vcs defines the pure domain traits (VcsRunner, ArgvRunner).
//! cynic-host provides the concrete local process execution adapters.

use cynic_vcs::{ArgvOutput, ArgvRunner, VcsCommandOutput, VcsRunner};

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalVcsRunner;

impl VcsRunner for LocalVcsRunner {
    fn run(&self, argv: &[&str]) -> VcsCommandOutput {
        // Migrated to ArgvRunner: single point for resource limits, timeouts, logging.
        match crate::adapters::argv_runner::ArgvRunner::run(argv) {
            Ok(output) => VcsCommandOutput {
                exit_code: output.exit_code,
                stdout: output.stdout.into_bytes(),
                stderr: output.stderr.into_bytes(),
                spawn_error: output.spawn_error,
            },
            Err(error) => VcsCommandOutput {
                exit_code: None,
                stdout: Vec::new(),
                stderr: Vec::new(),
                spawn_error: Some(error),
            },
        }
    }
}

impl ArgvRunner for LocalVcsRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        // Migrated to ArgvRunner: single point for resource limits, timeouts, logging.
        match crate::adapters::argv_runner::ArgvRunner::run(argv) {
            Ok(output) => ArgvOutput {
                exit_code: output.exit_code,
                stdout: output.stdout,
                stderr: output.stderr,
                spawn_error: output.spawn_error,
            },
            Err(error) => ArgvOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: Some(error),
            },
        }
    }
}
