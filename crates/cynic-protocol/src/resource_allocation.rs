//! Resource Allocation & Bursting Policy Contract (IaC Headroom Pattern)
//!
//! Enforces predictable resource guarantees while allowing temporary burst headroom
//! during resource-heavy workload execution (builds, LLM model swaps, indexing)
//! before automatic garbage collection / cleanup.

use serde::{Deserialize, Serialize};

pub const RESOURCE_BURST_POLICY_SCHEMA: &str = "cynic.resource_allocation_policy.v1";

/// Role-specific resource limits and temporary burst allowances.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeResourceLimits {
    pub schema: String,
    pub node_id: String,
    pub role: String,
    pub baseline_memory_bytes: u64,
    pub burst_memory_ceiling_bytes: u64,
    pub burst_ttl_seconds: u64,
    pub baseline_disk_scratch_bytes: u64,
    pub burst_disk_scratch_ceiling_bytes: u64,
    pub high_watermark_cleanup_threshold_pct: u8,
}

impl NodeResourceLimits {
    /// Canonical defaults for a dedicated build runner node.
    pub fn for_build_runner(node_id: impl Into<String>) -> Self {
        Self {
            schema: RESOURCE_BURST_POLICY_SCHEMA.to_string(),
            node_id: node_id.into(),
            role: "build_runner".to_string(),
            baseline_memory_bytes: 4 * 1024 * 1024 * 1024, // 4 GiB baseline
            burst_memory_ceiling_bytes: 16 * 1024 * 1024 * 1024, // 16 GiB burst headroom during compilation
            burst_ttl_seconds: 1800,                             // 30 minutes max burst window
            baseline_disk_scratch_bytes: 10 * 1024 * 1024 * 1024, // 10 GiB baseline
            burst_disk_scratch_ceiling_bytes: 30 * 1024 * 1024 * 1024, // 30 GiB burst scratch
            high_watermark_cleanup_threshold_pct: 75, // Pre-cleanup triggered at 75% disk usage
        }
    }

    /// Canonical defaults for an inference node.
    pub fn for_inference_node(node_id: impl Into<String>) -> Self {
        Self {
            schema: RESOURCE_BURST_POLICY_SCHEMA.to_string(),
            node_id: node_id.into(),
            role: "inference_node".to_string(),
            baseline_memory_bytes: 8 * 1024 * 1024 * 1024, // 8 GiB baseline
            burst_memory_ceiling_bytes: 24 * 1024 * 1024 * 1024, // 24 GiB burst for model load/swap
            burst_ttl_seconds: 600,                        // 10 minutes burst for model transitions
            baseline_disk_scratch_bytes: 20 * 1024 * 1024 * 1024,
            burst_disk_scratch_ceiling_bytes: 50 * 1024 * 1024 * 1024,
            high_watermark_cleanup_threshold_pct: 80,
        }
    }
}

/// Dynamic state of resource allocation and pre-emptive cleanup triggers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocationMode {
    Baseline,
    BurstActive {
        workload_id: String,
        started_at_unix_ms: u128,
        expires_at_unix_ms: u128,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceAllocationAssessment {
    pub node_id: String,
    pub mode: AllocationMode,
    pub memory_used_bytes: u64,
    pub memory_limit_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_utilization_pct: u8,
    pub preemptive_cleanup_required: bool,
    pub burst_expired: bool,
}

impl ResourceAllocationAssessment {
    pub fn evaluate(
        limits: &NodeResourceLimits,
        mode: AllocationMode,
        memory_used_bytes: u64,
        disk_used_bytes: u64,
        disk_total_bytes: u64,
        now_unix_ms: u128,
    ) -> Self {
        let disk_utilization_pct = if disk_total_bytes > 0 {
            ((disk_used_bytes as f64 / disk_total_bytes as f64) * 100.0).min(100.0) as u8
        } else {
            0
        };

        let preemptive_cleanup_required =
            disk_utilization_pct >= limits.high_watermark_cleanup_threshold_pct;

        let (memory_limit_bytes, burst_expired) = match &mode {
            AllocationMode::Baseline => (limits.baseline_memory_bytes, false),
            AllocationMode::BurstActive {
                expires_at_unix_ms, ..
            } => {
                if now_unix_ms >= *expires_at_unix_ms {
                    (limits.baseline_memory_bytes, true)
                } else {
                    (limits.burst_memory_ceiling_bytes, false)
                }
            }
        };

        Self {
            node_id: limits.node_id.clone(),
            mode,
            memory_used_bytes,
            memory_limit_bytes,
            disk_used_bytes,
            disk_total_bytes,
            disk_utilization_pct,
            preemptive_cleanup_required,
            burst_expired,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preemptive_cleanup_triggered_at_high_watermark() {
        let limits = NodeResourceLimits::for_build_runner("cynic-builder");
        // 80% usage on a 100GB disk (exceeds 75% high watermark)
        let assessment = ResourceAllocationAssessment::evaluate(
            &limits,
            AllocationMode::Baseline,
            2 * 1024 * 1024 * 1024,
            80 * 1024 * 1024 * 1024,
            100 * 1024 * 1024 * 1024,
            1_787_056_000_000,
        );

        assert!(assessment.preemptive_cleanup_required);
        assert_eq!(assessment.disk_utilization_pct, 80);
    }

    #[test]
    fn test_burst_headroom_increases_memory_limit() {
        let limits = NodeResourceLimits::for_build_runner("cynic-builder");
        let now_ms = 1_787_056_000_000;
        let mode = AllocationMode::BurstActive {
            workload_id: "build-001".to_string(),
            started_at_unix_ms: now_ms,
            expires_at_unix_ms: now_ms + 1_800_000,
        };

        let assessment = ResourceAllocationAssessment::evaluate(
            &limits,
            mode,
            6 * 1024 * 1024 * 1024,
            20 * 1024 * 1024 * 1024,
            100 * 1024 * 1024 * 1024,
            now_ms + 100_000,
        );

        assert_eq!(
            assessment.memory_limit_bytes,
            limits.burst_memory_ceiling_bytes
        );
        assert!(!assessment.burst_expired);
        assert!(!assessment.preemptive_cleanup_required);
    }
}
