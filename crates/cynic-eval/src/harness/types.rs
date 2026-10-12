use serde::{Deserialize, Serialize};

pub const BENCHMARK_SCHEMA: &str = "cynic.agent_harness.run.v1";
pub use cynic_protocol::{
    RuntimeCycleEnvelope as CycleEnvelope, RuntimePlaneLayer as RuntimeLayer,
};

fn default_true() -> bool {
    true
}

/// Canonicalize a Dog/agent runtime id for the agent harness.
///
/// NOTE: `mercury-agent` is NOT a CYNIC autonomous organ — it is the Cosmicstack
/// infra scheduler (separate project). The legacy alias `mercury-agent` was
/// historically normalized to `hermes-agent` (NousResearch) by mistake; that
/// mapping is removed here so the two frameworks are no longer conflated.
/// `hermes-agent` remains the canonical runtime id for the NousResearch
/// autonomous organ.
pub fn canonicalize_dog_runtime_id(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "hermes-agent" | "hermes_agent" => "hermes-agent".to_string(),
        _ => value.trim().to_string(),
    }
}

pub use canonicalize_dog_runtime_id as canonicalize_hermes_runtime_id;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimePlanSummary {
    pub alias: String,
    pub model_path: String,
    pub host: String,
    pub port: u16,
    pub ctx_size: u32,
    pub threads: u32,
    pub threads_batch: u32,
    pub n_gpu_layers: i32,
    pub flash_attn: String,
    pub memory_max: String,
    pub cpu_quota: String,
    pub kv_cache_type_k: Option<String>,
    pub kv_cache_type_v: Option<String>,
    pub batch_size: Option<u32>,
    pub ubatch_size: Option<u32>,
    pub rationale: Vec<String>,
    pub overrides_applied: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Audit,
    Agentic,
    Coding,
    PromptOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    DirectLlama,
    HermesAgent,
    AgentOs,
    RemoteFramework,
}

impl Default for RuntimeKind {
    fn default() -> Self {
        Self::DirectLlama
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelParameters {
    pub model_id: String,
    pub model_path: String,
    /// Immutable content identity required when a run is compared to a baseline.
    #[serde(default)]
    pub model_digest: Option<String>,
    #[serde(default)]
    pub architecture: Option<String>,
    #[serde(default)]
    pub quantization: Option<String>,
    pub context_requested: u32,
    pub parallel_slots: u16,
    #[serde(default)]
    pub context_per_slot: Option<u32>,
    #[serde(default)]
    pub kv_cache_type_k: Option<String>,
    #[serde(default)]
    pub kv_cache_type_v: Option<String>,
    #[serde(default)]
    pub flash_attention: Option<bool>,
    #[serde(default)]
    pub gpu_layers: Option<i32>,
    #[serde(default)]
    pub fit_mode: Option<String>,
    #[serde(default)]
    pub threads: Option<u16>,
    #[serde(default)]
    pub batch_size: Option<u32>,
    #[serde(default)]
    pub ubatch_size: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub reasoning_budget: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelSpec {
    pub parameters: ModelParameters,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskSpec {
    pub task_id: String,
    pub kind: TaskKind,
    pub prompt: String,
    pub max_output_tokens: u32,
    pub expected_proof: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Thresholds {
    #[serde(default)]
    pub min_available_memory_bytes: Option<u64>,
    #[serde(default)]
    pub min_available_memory_percent: Option<f32>,
    #[serde(default)]
    pub max_rss_bytes: Option<u64>,
    #[serde(default)]
    pub max_rss_percent: Option<f32>,
    #[serde(default)]
    pub max_wall_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScenarioSpec {
    pub scenario_id: String,
    #[serde(default = "default_true")]
    pub probe_runtime_plan: bool,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub runtime_kind: RuntimeKind,
    #[serde(default)]
    pub runtime_id: Option<String>,
    #[serde(default)]
    pub runtime_version: Option<String>,
    #[serde(default)]
    pub coordination_kind: Option<String>,
    #[serde(default)]
    pub coordination_id: Option<String>,
    #[serde(default)]
    pub tool_count: Option<u16>,
    #[serde(default)]
    pub max_agent_steps: Option<u32>,
    pub model: ModelSpec,
    pub tasks: Vec<TaskSpec>,
    pub thresholds: Thresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MemorySample {
    pub rss_bytes: Option<u64>,
    pub available_memory_bytes: Option<u64>,
    #[serde(default)]
    pub total_memory_bytes: Option<u64>,
    #[serde(default)]
    pub available_memory_percent: Option<f32>,
    #[serde(default)]
    pub rss_percent: Option<f32>,
    pub vram_bytes: Option<u64>,
    pub gtt_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HarnessSample {
    pub task_id: String,
    #[serde(default)]
    pub runtime_run_id: Option<String>,
    pub http_status: Option<u16>,
    pub prompt_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub prompt_tokens_per_second: Option<f32>,
    pub output_tokens_per_second: Option<f32>,
    pub wall_time_ms: Option<u64>,
    #[serde(default)]
    pub event_count: Option<u32>,
    #[serde(default)]
    pub event_kinds: Vec<String>,
    #[serde(default)]
    pub event_stream_error: Option<String>,
    pub memory: MemorySample,
    pub effective_context_per_slot: Option<u32>,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentHarnessReport {
    pub schema: String,
    pub run_id: String,
    pub scenario: ScenarioSpec,
    pub samples: Vec<HarnessSample>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    #[serde(default)]
    pub cycle: Option<CycleEnvelope>,
    #[serde(default)]
    pub transition: Option<cynic_protocol::CycleTransitionRecord>,
    #[serde(default)]
    pub runtime_transition: Option<cynic_protocol::CycleTransitionRecord>,
    #[serde(default)]
    pub runtime_launch_plan: Option<RuntimePlanSummary>,
}
