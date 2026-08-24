//! Remote memory observation adapter (SSH).
//!
//! Drives the same pure parsers as `LocalMemoryAdapter` but over a remote host
//! via an injected `ArgvRunner` (so the adapter is unit-testable with a fake
//! runner). Conservative parity: reads `/proc/meminfo` and `/proc/pressure/memory`
//! over SSH; no per-node `free` fallback is assumed (meminfo is authoritative
//! and always present).

use crate::domain::memory::port::MemoryObservationError;
use crate::domain::memory::{
    MemoryObservationPort, parse_meminfo, parse_pressure_memory, types::MemoryReport,
};
use crate::update_lifecycle::{ArgvOutput, ArgvRunner};

pub struct RemoteMemoryAdapter<R: ArgvRunner> {
    runner: R,
    target: Vec<String>,
}

impl<R: ArgvRunner> RemoteMemoryAdapter<R> {
    pub fn new(runner: R, ssh_target: impl Into<String>) -> Self {
        Self {
            runner,
            target: vec!["ssh".to_string(), ssh_target.into()],
        }
    }

    fn capture_remote(&self, remote_path: &str) -> Option<String> {
        let mut argv: Vec<String> = self.target.clone();
        argv.push(format!("cat {remote_path}"));
        let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let out = self.runner.run(&refs);
        if out.success() && !out.stdout.is_empty() {
            Some(out.stdout)
        } else {
            None
        }
    }
}

impl<R: ArgvRunner> MemoryObservationPort for RemoteMemoryAdapter<R> {
    fn observe_memory(&self) -> Result<MemoryReport, MemoryObservationError> {
        let meminfo_text = self.capture_remote("/proc/meminfo").ok_or_else(|| {
            MemoryObservationError::MeminfoRead("remote meminfo empty".to_string())
        })?;
        let mut report =
            parse_meminfo(&meminfo_text).map_err(MemoryObservationError::MeminfoRead)?;

        match self.capture_remote("/proc/pressure/memory") {
            Some(text) => match parse_pressure_memory(&text) {
                Ok(pressure) => report.pressure = Some(pressure),
                Err(err) => report
                    .residual_unknowns
                    .push(format!("pressure parse: {err}")),
            },
            None => report
                .residual_unknowns
                .push("pressure read: remote absent".to_string()),
        }

        Ok(report)
    }
}
