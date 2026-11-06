//! Memory observation sub-domain (physical → volume → usage → pressure).
//!
//! Canonical contract types are re-exported from `cynic_protocol`.

pub use cynic_protocol::host_substrate::memory::{
    FleetMemoryNode, FleetMemoryReport, MemoryPressure, MemoryReport, MemoryUsage, MemoryVolume,
};
