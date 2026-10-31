//! Backwards-compatible re-exports of build evidence adapters and ports.
//!
//! The implementations now live in `cynic-build`; this module keeps the
//! historical `cynic_host::build_evidence` paths working for existing
//! consumers.

pub use cynic_build::{
    BuildLeaseVerificationPort, BuildRunnerObservationPort, CurlBuildLeaseVerificationAdapter,
    JsonBuildRunnerObservationAdapter, NodeBuildLeaseVerificationAdapter,
    SshBuildRunnerObservationAdapter,
};
