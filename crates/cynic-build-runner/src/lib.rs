//! cynic-build-runner: native replacement for shell rsync→ssh→grep pipeline.
//!
//! Ports: BuildRunnerPort (cargo/check/rsync), SubstrateDiscovery (resolve host),
//! EvidenceObserver (proof emission). Adapters are injectable — no hardcoded hosts.
//!
//! One crate per cohesive functional domain (R7). Surface depends ONLY on
//! cynic-protocol vocabulary; no downward dependency on cynic-host/cynic-cli.

pub mod adapters;
pub mod decision;
pub mod ports;
pub mod surface;
pub mod types;

pub use ports::{BuildRunnerPort, EvidenceObserver, SubstrateDiscovery};
pub use types::{BuildProof, BuildRequest, BuildResult, ErrorCatalog, ErrorEntry, BuilderTarget};
