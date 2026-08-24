//! Memory observation sub-domain (physical → volume → usage → pressure).
//!
//! Canonical contract types live in `cynic-protocol/src/memory.rs`. This
//! module re-exports them and hosts the pure parsers (`probe`) and the
//! observation port (`port`) that adapters implement.

pub mod port;
pub mod probe;
pub mod types;

pub use port::MemoryObservationPort;
pub use probe::{parse_free_output, parse_meminfo, parse_pressure_memory};
pub use types::{
    bytes_to_human, FleetMemoryNode, FleetMemoryReport, MemoryPressure, MemoryReport, MemoryUsage,
    MemoryVolume,
};
