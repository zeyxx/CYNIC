//! cynic-build: sovereign build pipeline bounded context.
//!
//! Hexagonal layout:
//! - types.rs — pure vocabulary (requests, proofs, evidence).
//! - ports.rs — driven ports (command execution, filesystem, identity).
//! - runner.rs — pipeline decisions over injected ports. No OS calls.
//! - adapters/ — OS-backed port implementations plus an in-memory fake.
//! - build/, build_*, release_publication, runtime_release — pure
//!   admission, scheduling, and publication contracts.
//! - compressor — pure proof compression.
//!
//! This root is a thin composition surface: module declarations and
//! re-exports only. No logic, no I/O.

pub mod adapters;
pub mod builder_restart;
pub mod admin_signal;
pub mod build;
pub mod cache;
pub mod build_artefact;
pub mod build_disk_capacity;
pub mod build_memory_capacity;
pub mod build_discovery;
pub mod domain;
pub mod build_discovery_recipe;
pub mod build_node;
pub mod toolchain;
pub mod build_proof;
pub mod build_queue;
pub mod build_reliability;
pub mod build_runner_harness;
pub mod build_slice;
pub mod build_scheduler;
pub mod build_workflow;
pub mod builder_client;
pub mod builder_rbac;
pub mod builder_pool;
pub mod collision_counter;
pub mod compressor;
pub mod evidence;
pub mod external_build;
#[deprecated(note = "use cynic_build::external_build directly")]
pub mod external_build_uc;
pub mod gate_scope;
pub mod gc;
pub mod harness;
pub mod harness_authority;
pub mod machine_deploy_exec;
pub mod place;
pub mod pipeline_proof;
pub mod ports;
pub mod release_publication;
pub mod runner;
pub mod runtime_release;
pub mod types;

pub use adapters::{CurlBuildLeaseVerificationAdapter, HttpDeployAdapter, HttpExternalBuildAdapter, JsonBuildRunnerObservationAdapter, LocalReleaseBasketPackaging, MemFilesystem, NodeBuildLeaseVerificationAdapter, OsIdentity, SshBuildRunnerObservationAdapter, SshReleaseBasketSigning, StdFilesystem};
pub use adapters::{
    DEFAULT_SSOT_URL, HttpArtifactTransport, LocalBuildArtefactAdapter, LocalBuildWorkspace,
    LocalQueueAdapter, SshBuildRunner, TokioBuildExecution,
};
pub use build::*;
pub use cache::*;
pub use build_artefact::*;
pub use build_disk_capacity::*;
pub use build_memory_capacity::*;
pub use build_discovery::*;
pub use build_discovery_recipe::*;
pub use build_node::*;
pub use build_proof::*;
pub use build_queue::*;
pub use build_reliability::*;
pub use build_runner_harness::*;
pub use build_slice::*;
pub use build_workflow::*;
pub use build_scheduler::config::{BuildSchedulerConfig, NodeControlConfig};
pub use build_scheduler::grant::issue_signed_build_workload_grant;
pub use build_scheduler::release_publication::publish_with_ports;
pub use ports::{
    BuildArtefactReclaimer, CapacityAssessment, CapacityAssessmentPort, CpuMeterPort, CpuSample,
    LoadPolicy, LoadTier, RuntimeArtifactTransportPort, WorkspaceLayoutResolver,
};
pub use build_scheduler::config::ReleasePublicationConfig;
pub use builder_client::*;
pub use builder_rbac::*;
pub use collision_counter::*;
pub use compressor::*;
pub use evidence::*;
pub use external_build::*;
pub use external_build_uc::*;
pub use toolchain::*;
pub use place::*;
pub use harness::*;
pub use harness_authority::*;
pub use machine_deploy_exec::*;
pub use pipeline_proof::*;
pub use ports::{BuildCommandOutput, BuildExecutionError, BuildExecutionPort, BuildExecutionReadBack, BuildExecutionRequest, BuildRunner, BuildWorkspacePort, BuildWorkspaceReadBack, DetachedReleaseSignature, FilesystemPort, IdentityPort, PackagedReleaseBasket, ReleaseArtifactPublicationPort, ReleaseBasketPackagingPort, ReleaseBasketSigningPort, ReleasePublicationError, ReleasePublicationFuture, ReleasePublicationResult, ReleasePublicationTargets};
pub use release_publication::*;
pub use runner::{install_artifact, run_build};
pub use runtime_release::*;
pub use types::*;
