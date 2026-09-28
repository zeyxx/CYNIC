//! Local capacity assessment adapter — assembles CPU + memory canonical
//! reports and evaluates the host load tier under a node-declared policy.
//!
//! Thin composition layer: reuses `LocalCpuAdapter`/`LocalMemoryAdapter`
//! observations (no re-measurement) and `cynic_protocol::capacity` pure
//! decision. Read-only; never fabricates HighPerformance without evidence.

use cynic_protocol::capacity::{CapacityAssessmentV1, LoadPolicy, assess_load_tier};

use crate::adapters::local_cpu::LocalCpuAdapter;
use crate::adapters::local_memory::LocalMemoryAdapter;
use crate::domain::cpu::port::CpuObservationPort;
use crate::domain::memory::MemoryObservationPort;

pub struct LocalCapacityAdapter;

impl LocalCapacityAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Observe local CPU and memory, then assess the load tier.
    /// Missing probes become explicit residual unknowns — never fabricated.
    pub fn assess_capacity(
        &self,
        policy: &LoadPolicy,
    ) -> Result<CapacityAssessmentV1, String> {
        let policy_violations = policy.violations();
        if !policy_violations.is_empty() {
            return Err(format!(
                "load_policy_invalid: {}",
                policy_violations.join(",")
            ));
        }

        let cpu_report = LocalCpuAdapter
            .observe_cpu()
            .map_err(|e| format!("cpu observation failed: {e}"))
            .ok();
        let memory_report = LocalMemoryAdapter
            .observe_memory()
            .map_err(|e| format!("memory observation failed: {e}"))
            .ok();

        Ok(assess_load_tier(
            cpu_report.as_ref(),
            memory_report.as_ref(),
            policy,
        ))
    }
}

impl Default for LocalCapacityAdapter {
    fn default() -> Self {
        Self::new()
    }
}
