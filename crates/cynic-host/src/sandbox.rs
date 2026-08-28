//! Sandbox substrate adapters: bwrap, netns, egress proxy, RAPL energy.
//!
//! Each adapter exposes a trait. Concrete implementations live behind
//! platform-specific gates (Linux netlink, RAPL sysfs, etc.).

pub mod bwrap;
pub mod egress_proxy;
pub mod netns;
pub mod package;
pub mod rapl;
