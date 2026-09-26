//! Durable port contracts — adapters are disposable implementations.

use crate::types::{BuildResult, BuildProof, BuilderTarget};

pub trait BuildRunnerPort: Send + Sync {
    fn rsync_to(&self, src: &str, host: &str, dest: &str) -> Result<(), String>;
    fn cargo_check(&self, manifest: &str, packages: &[String]) -> BuildResult;
    fn extract_errors(&self, log: &str) -> crate::types::ErrorCatalog;
    fn run_build(&self, plan: &crate::decision::BuildPlan) -> BuildResult {
        self.rsync_to(&plan.src, &plan.builder.host, &plan.builder.workdir)?;
        self.cargo_check(&plan.manifest, &plan.packages)
    }
}
pub trait EvidenceObserver: Send + Sync {
    fn emit(&self, proof: BuildProof);
}
pub trait SubstrateDiscovery: Send + Sync {
    fn resolve(&self) -> BuilderTarget;
}
