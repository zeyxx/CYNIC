//! Injectables for unit tests — return fixed JSON, never hit network.
use crate::ports::*;
use crate::types::*;
pub struct StubRunner;
impl BuildRunnerPort for StubRunner {
    fn rsync_to(&self, _: &str, _: &str, _: &str) -> Result<(), String> { Ok(()) }
    fn cargo_check(&self, _: &str, _: &[String]) -> BuildResult {
        BuildResult { exit_code: 0, errors: ErrorCatalog::default(), log_ref: "stub".into() }
    }
    fn extract_errors(&self, _: &str) -> ErrorCatalog { ErrorCatalog::default() }
}
impl EvidenceObserver for StubRunner {
    fn emit(&self, _: BuildProof) {}
}
impl SubstrateDiscovery for StubRunner {
    fn resolve(&self) -> BuilderTarget { BuilderTarget { host: "stub".into(), workdir: "/tmp".into() } }
}
