//! Production adapters: OS-backed implementations of the driven ports.
//!
//! Only this directory (and tests) may touch `std::fs`, `libc`, or the
//! process table. Everything else in the crate programs against ports.rs.

pub mod build_evidence;
pub mod http;
pub mod local;
pub mod local_build_cache;
pub mod local_grant_store;
pub mod mem_fs;
pub mod os_identity;
pub mod std_fs;
pub mod local_release_basket_packaging;
pub mod ssh_release_basket_signing;
pub mod agent_execution_runtime_observer;


pub mod ssh_build_runner;
pub mod local_build_workspace;
pub mod tokio_build_execution;

pub use build_evidence::{
    CurlBuildLeaseVerificationAdapter, JsonBuildRunnerObservationAdapter,
    NodeBuildLeaseVerificationAdapter, SshBuildRunnerObservationAdapter,
};

pub use ssh_build_runner::{DEFAULT_SSOT_URL, SshBuildRunner};
pub use local_build_workspace::LocalBuildWorkspace;
pub use tokio_build_execution::TokioBuildExecution;

pub use http::{HttpDeployAdapter, HttpExternalBuildAdapter};
pub use local::{LocalExternalBuildAdapter};
pub use local_build_cache::LocalBuildCacheAdapter;
pub use local_grant_store::LocalBuildWorkloadGrantStore;
pub use mem_fs::MemFilesystem;
pub use os_identity::OsIdentity;
pub use std_fs::StdFilesystem;
pub use local_release_basket_packaging::*;
pub use ssh_release_basket_signing::*;

pub mod build_discovery;
pub use build_discovery::{LocalBuildDiscoveryAdapter, RemoteBuildDiscoveryAdapter};
