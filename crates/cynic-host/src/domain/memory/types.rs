//! Shared vocabulary re-exported for the memory domain.
//!
//! Pure contract types live in `cynic-protocol/src/memory.rs` so that the
//! kernel, node, host, and CLI surfaces all reference one canonical definition
//! with never a circular dependency. This file is a thin re-export shim, kept
//! separate so the `domain/memory` module tree can grow probe/port/plan
//! submodules without touching the protocol crate.
//!
//! Note: `bytes_to_human` lives in `cynic_protocol::disk` (the single canonical
//! helper shared by every domain), not in `memory`.

pub use cynic_protocol::disk::bytes_to_human;
pub use cynic_protocol::memory::{
    FleetMemoryNode, FleetMemoryReport, MemoryPressure, MemoryReport, MemoryUsage, MemoryVolume,
};
