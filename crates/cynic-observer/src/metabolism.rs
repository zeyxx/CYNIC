//! System Metabolism Provider for Observer
//!
//! Observes host hardware usage (CPU and memory) to determine the node's
//! metabolic operating state (HighPerformance, Standard, Survival) without
//! depending on host orchestrator crates.

use cynic_protocol::capacity::LoadTier;

pub trait LoadProbe: Send + Sync {
    /// Returns the host's current load tier.
    fn current_load(&mut self) -> LoadTier;
}

/// OS-backed load probe using `sysinfo`.
pub struct OsLoadProbe {
    system: sysinfo::System,
}

impl std::fmt::Debug for OsLoadProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OsLoadProbe").finish()
    }
}

impl OsLoadProbe {
    pub fn new() -> Self {
        let mut system = sysinfo::System::new_all();
        system.refresh_all();
        Self { system }
    }
}

impl Default for OsLoadProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadProbe for OsLoadProbe {
    fn current_load(&mut self) -> LoadTier {
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
            LoadTier::Saturated
        } else if global_cpu > 40.0 || mem_usage_percent > 60.0 {
            LoadTier::Normal
        } else {
            LoadTier::Headroom
        }
    }
}
