//! Read-only observation port for the memory vertical.
//!
//! Port = pure trait naming WHAT can be observed, never HOW (that is the
//! adapter's job). Adapters (`LocalMemoryAdapter`, `RemoteMemoryAdapter`)
//! implement this and are injected by the CLI/host surface.

use crate::domain::memory::types::MemoryReport;

pub trait MemoryObservationPort {
    /// Produce a full 4-layer memory report (physical/volume/usage/pressure).
    /// Returns a structured report + optional residual-unknowns list.
    fn observe_memory(&self) -> Result<MemoryReport, MemoryObservationError>;
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryObservationError {
    #[error("meminfo read failed: {0}")]
    MeminfoRead(String),
    #[error("free output parse failed: {0}")]
    FreeParse(String),
    #[error("pressure read failed: {0}")]
    Pressure(String),
    #[error("admission: {0}")]
    Admission(String),
}
