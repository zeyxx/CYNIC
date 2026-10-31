//! Production adapters: OS-backed implementations of the driven ports.
//!
//! Only this directory (and tests) may touch `std::fs`, `sysinfo`, or the
//! process table. Everything else in the crate programs against ports.rs.

pub mod host;

pub use host::{
    HostActionMeter, HostComputeMeter, HostEnergyMeter, HostNetworkMeter, HostStorageMeter,
    HostTimeMeter,
};
