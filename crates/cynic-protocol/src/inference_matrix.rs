//! Typed schemas for the CYNIC Multi-Model Parametric Benchmark Matrix & Harness Sweeps.
//!
//! Enables exhaustive evaluation across context scaling (4k -> 128k), KV quantization,
//! thread pinning, batch sizes, and multi-topology execution (Co-Resident vs Hot-Swap).

use serde::{Deserialize, Serialize};

pub const INFERENCE_MATRIX_SWEEP_SCHEMA: &str = "cynic.inference.matrix_sweep.v2";
pub const INFERENCE_HARNESS_PROFILE_SCHEMA: &str = "cynic.inference.harness_profile.v1";
pub const INFERENCE_SCORECARD_SCHEMA: &str = "cynic.inference.scorecard.v1";
pub const INFERENCE_OPTIMAL_PROFILE_SCHEMA: &str = "cynic.inference.optimal_profile.v1";

/// Target context length tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ContextTier {
    #[serde(rename = "4k")]
    Ctx4k = 4096,
    #[serde(rename = "8k")]
    Ctx8k = 8192,
    #[serde(rename = "16k")]
    Ctx16k = 16384,
    #[serde(rename = "32k")]
    Ctx32k = 32768,
    #[serde(rename = "64k")]
    Ctx64k = 65536,
    #[serde(rename = "128k")]
    Ctx128k = 131072,
}

impl ContextTier {
    pub const fn token_count(self) -> u32 {
        self as u32
    }
}

/// KV-Cache precision strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KvQuantization {
    Fp16,
    Q8_0,
    Q4_0,
    Q4_1,
}

impl KvQuantization {
    /// Bytes per element in KV cache.
    pub const fn bytes_per_element(self) -> f32 {
        match self {
            Self::Fp16 => 2.0,
            Self::Q8_0 => 1.0,
            Self::Q4_0 | Self::Q4_1 => 0.5,
        }
    }
}

/// Multi-model topology in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyStrategy {
    /// 1 single heavy model utilizing up to 100% of RAM/VRAM budget.
    DedicatedSingle,
    /// 2+ models active concurrently in RAM (e.g. 1B router + 14B worker).
    CoResident,
    /// Fast event-driven swapping from NVMe mmap (< 4s switch).
    HotSwapNvme,
}

/// Agent communication and tool calling harness protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessProtocol {
    /// Standard /v1/chat/completions with tools JSON parameter.
    OpenAiToolCall,
    /// Hermes XML <tool_call> tags.
    HermesXml,
    /// Line-anchored patch grammar (OMP / Claude Code diffs).
    OmpLinePatch,
    /// Mercury UDS typed message passing.
    MercuryJson,
}

/// RoPE scaling configuration for extreme context (>32k).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RopeScaling {
    None,
    Linear { factor: f32 },
    Yarn { freq_scale: f32, freq_base: f32 },
}

/// A fully specified execution configuration point in the parametric grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametricConfigPoint {
    pub model_alias: String,
    pub model_file: String,
    pub context_tier: ContextTier,
    pub kv_quant: KvQuantization,
    pub threads_gen: u32,
    pub threads_batch: u32,
    pub batch_size: u32,
    pub ubatch_size: u32,
    pub flash_attn: bool,
    pub gpu_layers: u32,
    pub rope_scaling: RopeScaling,
    pub topology: TopologyStrategy,
    pub target_harness: HarnessProtocol,
}

/// Empirical result of running one configuration point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametricRunResult {
    pub config: ParametricConfigPoint,
    pub cold_load_secs: f32,
    pub ram_peak_bytes: u64,
    pub prompt_tok_per_sec: f32,
    pub gen_tok_per_sec: f32,
    pub ttft_ms: f32,
    pub tool_schema_conformity: f32,
    pub compilability_ratio: f32,
    pub epistemic_score: f32,
    pub success: bool,
    pub error: Option<String>,
}

/// Aggregated scorecard across an entire matrix sweep.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixScorecard {
    pub schema: &'static str,
    pub hardware_signature: String,
    pub total_configurations_tested: usize,
    pub passed_configurations: usize,
    pub top_throughput_point: Option<ParametricRunResult>,
    pub top_epistemic_point: Option<ParametricRunResult>,
    pub recommended_64k_profile: Option<ParametricConfigPoint>,
    pub recommended_coresident_profile: Option<Vec<ParametricConfigPoint>>,
}

impl MatrixScorecard {
    pub const SCHEMA: &'static str = INFERENCE_SCORECARD_SCHEMA;
}
