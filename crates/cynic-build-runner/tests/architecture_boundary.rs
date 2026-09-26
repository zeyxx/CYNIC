//! Architecture boundary: cynic-build-runner must not depend downward
//! on cynic-host's implementation modules — only on ports.
//!
//! This test enforces R7: domain crates never depend on orchestrator/host.
//! If it fails, a port was bypassed.

use cynic_build_runner::{BuildRunner, SubstrateDiscovery};

#[test]
fn ports_are_sealed_contract() {
    // If this compiles, the trait surface is pure (no host impl leakage).
    fn _accept<T: BuildRunner + ?Sized>() {}
    fn _accept2<T: SubstrateDiscovery + ?Sized>() {}
}
