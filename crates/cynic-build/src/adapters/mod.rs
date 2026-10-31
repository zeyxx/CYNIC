//! Production adapters: OS-backed implementations of the driven ports.
//!
//! Only this directory (and tests) may touch `std::fs`, `libc`, or the
//! process table. Everything else in the crate programs against ports.rs.

pub mod http;
pub mod local;
pub mod local_build_cache;
pub mod mem_fs;
pub mod os_identity;
pub mod std_fs;

pub use http::{HttpDeployAdapter, HttpDeploy, HttpDeploymentAdapter, HttpDeployPortAdapter, HttpDeployer, HttpExternalBuildAdapter};
pub use local::{LocalExternalBuildAdapter};
pub use local_build_cache::LocalBuildCacheAdapter;
pub use mem_fs::MemFilesystem;
pub use os_identity::OsIdentity;
pub use std_fs::StdFilesystem;
