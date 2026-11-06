//! Substrate OS process execution and in-memory test runner adapters
//! implementing [`cynic_protocol::argv::ArgvRunner`].

pub mod local;
pub mod mock;
pub mod process;

pub use local::LocalArgvRunner;
pub use mock::MapArgvRunner;

pub use cynic_protocol::argv::{ArgvOutput, ArgvRunner};
