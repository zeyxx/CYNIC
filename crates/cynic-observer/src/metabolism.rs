//! System Metabolism Provider for Observer
//!
//! Observes host hardware usage (CPU and memory) to determine the node's
//! metabolic operating state (HighPerformance, Standard, Survival) without
//! depending on host orchestrator crates.

use cynic_protocol::capacity::MetabolicState;

pub trait MetabolismProvider: Send + Sync {
    /// Returns the current metabolic state of the host system.
    fn current_state(&mut self) -> MetabolicState;
}

/// OS-backed metabolism probe using sysinfo.
pub struct OsMetabolism {
    system: sysinfo::System,
}

impl std::fmt::Debug for OsMetabolism {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OsMetabolism").finish()
    }
}

impl OsMetabolism {
    pub fn new() -> Self {
        let mut system = sysinfo::System::new_all();
        system.refresh_all();
        Self { system }
    }
}

impl Default for OsMetabolism {
    fn default() -> Self {
        Self::new()
    }
}

impl MetabolismProvider for OsMetabolism {
    fn current_state(&mut self) -> MetabolicState {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        let global_cpu = self.system.global_cpu_usage();
        let free_memory = self.system.free_memory();
        let total_memory = self.system.total_memory();
        let mem_usage_percent = if total_memory > 0 {
            ((total_memory - free_memory) as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        };

        if global_cpu > 85.0 || mem_usage_percent > 90.0 {
            MetabolicState::Survival
        } else if global_cpu > 40.0 || mem_usage_percent > 60.0 {
            MetabolicState::Standard
        } else {
            MetabolicState::HighPerformance
        }
    }
}
