//! Local memory observation adapter.
//!
//! Reads the kernel's `/proc` substrate directly (no external host dependency)
//! and assembles the 4-layer `MemoryReport` via the pure parsers in
//! `domain::memory::probe`. This is the localhost-observation surface for the
//! host/node layer; fleet-wide observation is done through `FleetMemoryObserver`
//! which drives either this adapter (local) or `RemoteMemoryAdapter` (SSH).

use std::fs;
use std::path::Path;

use crate::domain::memory::types::MemoryReport;
use crate::domain::memory::{parse_meminfo, parse_pressure_memory, MemoryObservationPort};

pub struct LocalMemoryAdapter;

impl LocalMemoryAdapter {
    pub fn new() -> Self {
        Self
    }

    fn read_to_string(path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("{path:?} read failed: {error}"))
    }
}

impl MemoryObservationPort for LocalMemoryAdapter {
    fn observe_memory(&self) -> Result<MemoryReport, crate::domain::memory::port::MemoryObservationError> {
        let meminfo = Self::read_to_string(Path::new("/proc/meminfo"))
            .map_err(crate::domain::memory::port::MemoryObservationError::MeminfoRead)?;
        let report = parse_meminfo(&meminfo)
            .map_err(crate::domain::memory::port::MemoryObservationError::MeminfoRead)?;

        // Layer 4: pressure stalls (best-effort — unknown stays unknown).
        let report = match Self::read_to_string(Path::new("/proc/pressure/memory")) {
            Ok(pressure_text) => match parse_pressure_memory(&pressure_text) {
                Ok(pressure) => {
                    let mut r = report;
                    r.pressure = Some(pressure);
                    r
                }
                Err(err) => {
                    let mut r = report;
                    r.residual_unknowns.push(format!("pressure parse: {err}"));
                    r
                }
            },
            Err(err) => {
                let mut r = report;
                r.residual_unknowns.push(format!("pressure read: {err}"));
                r
            }
        };

        Ok(report)
    }
}

impl Default for LocalMemoryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: ensure the adapter binds the port trait.
    fn _assert_port<T: MemoryObservationPort>() {}
    #[test]
    fn local_memory_adapts_observation_port() {
        _assert_port::<LocalMemoryAdapter>();
    }
}
