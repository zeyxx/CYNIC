//! Bounded argv execution adapter for ecosystem package plans.

use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageCommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub spawn_error: Option<String>,
}

impl PackageCommandOutput {
    pub fn success(&self) -> bool {
        self.spawn_error.is_none() && self.exit_code == Some(0)
    }
}

pub trait EcosystemPackageExecutionPort: Send + Sync {
    fn execute(&self, argv: &[String], cwd: Option<&Path>) -> PackageCommandOutput;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalEcosystemPackageExecution;

impl EcosystemPackageExecutionPort for LocalEcosystemPackageExecution {
    fn execute(&self, argv: &[String], cwd: Option<&Path>) -> PackageCommandOutput {
        let Some(program) = argv.first() else {
            return PackageCommandOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: Some("empty argv".to_string()),
            };
        };
        let mut command = Command::new(program);
        command.args(&argv[1..]);
        if let Some(directory) = cwd.filter(|path| path.is_dir()) {
            command.current_dir(directory);
        }
        match command.output() {
            Ok(output) => PackageCommandOutput {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                spawn_error: None,
            },
            Err(error) => PackageCommandOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: Some(format!("spawn {program}: {error}")),
            },
        }
    }
}
