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
pub mod build;
pub mod build_artefact;
pub mod build_discovery;
pub mod build_discovery_recipe;
pub mod build_node;
pub mod build_proof;
pub mod build_queue;
pub mod build_reliability;
pub mod build_slice;
pub mod build_workflow;
pub mod compressor;
pub mod ports;
pub mod release_publication;
pub mod runner;
pub mod runtime_release;
pub mod types;

pub use adapters::{MemFilesystem, OsIdentity, StdFilesystem};
pub use build::*;
pub use build_artefact::*;
pub use build_discovery::*;
pub use build_discovery_recipe::*;
pub use build_node::*;
pub use build_proof::*;
pub use build_queue::*;
pub use build_reliability::*;
pub use build_slice::*;
pub use build_workflow::*;
pub use ports::{BuildCommandOutput, BuildRunner, FilesystemPort, IdentityPort};
pub use release_publication::*;
pub use runner::{install_artifact, run_build};
pub use runtime_release::*;
pub use types::*;
