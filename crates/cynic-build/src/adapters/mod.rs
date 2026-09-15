//! Production adapters: OS-backed implementations of the driven ports.
//!
//! Only this directory (and tests) may touch `std::fs`, `libc`, or the
//! process table. Everything else in the crate programs against ports.rs.

pub mod mem_fs;
pub mod os_identity;
pub mod std_fs;

pub use mem_fs::MemFilesystem;
pub use os_identity::OsIdentity;
pub use std_fs::StdFilesystem;
