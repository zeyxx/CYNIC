//! Pure domain types for the CYNIC Multi-Model Parametric Sweep & Benchmark Engine.

use cynic_protocol::{HarnessProtocol, TopologyStrategy};
use serde::{Deserialize, Serialize};

/// Concurrency configuration for running parallel sessions on the same model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelConcurrencySlots {
    /// Number of parallel slots allocated in llama-server (-np / --parallel).
    pub parallel_slots: u32,
    /// Whether KV cache is unified across slots or dedicated per slot.
    pub kv_unified: bool,
}

impl Default for ModelConcurrencySlots {
    fn default() -> Self {
        Self {
            parallel_slots: 1,
            kv_unified: true,
        }
    }
}

/// Observed hardware budget; populated at runtime from /proc/meminfo or system probes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareBudget {
    pub total_ram_bytes: u64,
    pub available_ram_bytes: u64,
    pub gpu_vram_bytes: u64,
}

impl HardwareBudget {
    /// Assemble from already-observed inputs (pure — no I/O, no env).
    ///
    /// `/proc/meminfo` text and the `CYNIC_GPU_VRAM_BYTES` override are read
    /// by the probe adapter and passed in.
    pub fn from_observed(meminfo_text: &str, gpu_vram_bytes: u64) -> Self {
        let (mut total, mut available) =
            cynic_types::proc_text::parse_meminfo_bytes(meminfo_text);
        // Fallback: assume 16 GiB if meminfo text carries no usable lines.
        if total == 0 {
            total = 16 * 1024 * 1024 * 1024;
        }
        if available == 0 {
            available = total / 2;
        }
        Self {
            total_ram_bytes: total,
            available_ram_bytes: available,
            gpu_vram_bytes,
        }
    }
}

/// A comprehensive test scenario executed against a running model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkScenario {
    pub id: String,
    pub domain: String,
    pub prompt: String,
    pub max_tokens: u32,
    pub protocol: HarnessProtocol,
    pub expected_schema: Option<serde_json::Value>,
    pub require_compilation: bool,
}

/// Measured streaming and token-emission dynamics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamDynamicsReport {
    /// Time to first emitted chunk / token in milliseconds.
    pub ttft_ms: f32,
    /// Mean inter-token emission interval in milliseconds.
    pub mean_inter_token_ms: f32,
    /// Variance / jitter of inter-token intervals in milliseconds.
    pub jitter_variance_ms: f32,
    /// Overall generation throughput (predicted tokens / second).
    pub predicted_tokens_per_second: f32,
    /// Prompt ingestion throughput (prompt tokens / second).
    pub prompt_tokens_per_second: f32,
}

/// Empirical assessment of a multi-model topology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyAssessment {
    pub strategy: TopologyStrategy,
    pub active_models: Vec<String>,
    pub total_ram_footprint_bytes: u64,
    pub swap_overhead_secs: Option<f32>,
    pub concurrency_throughput_agg_tok_sec: f32,
    pub meets_fleet_safety_budget: bool,
}
