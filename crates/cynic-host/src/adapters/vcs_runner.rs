//! Host-level implementation of VCS runners.
//!
//! cynic-vcs defines the pure domain traits (VcsRunner, ArgvRunner).
//! cynic-host provides the concrete local process execution adapters.

use cynic_vcs::{ArgvOutput, ArgvRunner, VcsCommandOutput, VcsRunner};

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalVcsRunner;

impl VcsRunner for LocalVcsRunner {
    fn run(&self, argv: &[&str]) -> VcsCommandOutput {
        if argv.is_empty() {
            return VcsCommandOutput {
                exit_code: None,
                stdout: Vec::new(),
                stderr: Vec::new(),
                spawn_error: Some("empty argv".to_string()),
            };
        }
        match std::process::Command::new(argv[0])
            .args(&argv[1..])
            .output()
        {
            Ok(output) => VcsCommandOutput {
                exit_code: output.status.code(),
                stdout: output.stdout,
                stderr: output.stderr,
                spawn_error: None,
            },
            Err(error) => VcsCommandOutput {
                exit_code: None,
                stdout: Vec::new(),
                stderr: Vec::new(),
                spawn_error: Some(error.to_string()),
            },
        }
    }
}

impl ArgvRunner for LocalVcsRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        if argv.is_empty() {
            return ArgvOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: Some("empty argv".to_string()),
            };
        }
        match std::process::Command::new(argv[0])
            .args(&argv[1..])
            .output()
        {
            Ok(output) => ArgvOutput {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                spawn_error: None,
            },
            Err(error) => ArgvOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: Some(error.to_string()),
            },
        }
    }
}
