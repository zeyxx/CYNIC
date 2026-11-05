//! Memory observation sub-domain (physical → volume → usage → pressure).
//!
//! Canonical contract types live in `cynic-protocol/src/memory.rs`. This
//! module re-exports them and hosts the pure parsers (`probe`) and the
//! observation port (`port`) that adapters implement.

pub mod port;
pub mod probe;

pub use port::MemoryObservationPort;
pub use probe::{parse_free_available_bytes, parse_free_output, parse_free_total_bytes, parse_meminfo, parse_pressure_memory};
pub use cynic_protocol::host_substrate::memory::{
    FleetMemoryNode, FleetMemoryReport, MemoryPressure, MemoryReport, MemoryUsage, MemoryVolume,
};
