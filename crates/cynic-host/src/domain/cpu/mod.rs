//! CPU observation domain (4-layer: physical / volume / usage / pressure).
//!
//! Pure parsers live in `probe`; the observe port is in `port`; canonical
//! types are re-exported from `cynic_protocol::cpu`.

pub mod port;
pub mod probe;
pub mod types;

pub use probe::{parse_cpuinfo, parse_pressure_cpu, parse_stat};
// Full canonical type surface (CpuUsage/CpuVolume/CpuPressure included) —
// adapters and probes reference them via `crate::domain::cpu::`.
pub use types::*;
