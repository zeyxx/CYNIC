//! Bounded argv execution adapter for ecosystem package plans.

use std::path::Path;
use cynic_process::LocalArgvRunner;
pub use cynic_protocol::argv::ArgvOutput as PackageCommandOutput;

pub trait EcosystemPackageExecutionPort: Send + Sync {
    fn execute(&self, argv: &[String], cwd: Option<&Path>) -> PackageCommandOutput;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalEcosystemPackageExecution;

impl EcosystemPackageExecutionPort for LocalEcosystemPackageExecution {
    fn execute(&self, argv: &[String], cwd: Option<&Path>) -> PackageCommandOutput {
        let str_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        LocalArgvRunner.run_in_dir(&str_refs, cwd)
    }
}
