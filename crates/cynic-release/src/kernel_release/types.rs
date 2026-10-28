pub use cynic_protocol::{
    RuntimeArtifactReleaseAction as KernelReleaseAction,
    RuntimeArtifactReleaseEvent as KernelReleaseEvent,
    RuntimeArtifactReleaseReport as KernelReleaseReport,
    RuntimeArtifactReleaseSpec as KernelReleaseSpec, RuntimeArtifactSpec as KernelArtifactSpec,
    RuntimeBuildEvidence as KernelBuildEvidence, ServiceIdentityEvidence as KernelIdentityEvidence,
};

pub const KERNEL_RELEASE_SPEC_SCHEMA: &str = cynic_protocol::RUNTIME_ARTIFACT_RELEASE_SPEC_SCHEMA;
pub const KERNEL_RELEASE_REPORT_SCHEMA: &str =
    cynic_protocol::RUNTIME_ARTIFACT_RELEASE_REPORT_SCHEMA;
pub const KERNEL_RELEASE_EVENT_SCHEMA: &str = cynic_protocol::RUNTIME_ARTIFACT_RELEASE_EVENT_SCHEMA;
