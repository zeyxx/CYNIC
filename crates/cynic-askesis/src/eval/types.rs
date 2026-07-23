use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationPolicy {
    pub name: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub min_p: f32,
    pub repeat_penalty: f32,
    pub presence_penalty: f32,
    pub frequency_penalty: f32,
    pub seed: i64,
    pub enable_thinking: bool,
    pub json_mode: bool,
    pub timeout_secs: u64,
    #[serde(default)]
    pub stop: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextOracle {
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub required_all: Vec<String>,
    #[serde(default)]
    pub required_any: Vec<Vec<String>>,
    #[serde(default)]
    pub forbidden: Vec<String>,
    pub max_response_chars: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub fixture_result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolOracle {
    #[serde(default)]
    pub required_tools: Vec<String>,
    #[serde(default)]
    pub forbidden_tools: Vec<String>,
    pub min_executions: Option<usize>,
    pub max_executions: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalCase {
    pub schema: String,
    pub id: String,
    pub domain: String,
    pub prompt: String,
    pub system: Option<String>,
    pub required_context_tokens: u32,
    pub policy: GenerationPolicy,
    pub oracle: TextOracle,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
    #[serde(default)]
    pub max_tool_turns: u32,
    pub tool_oracle: Option<ToolOracle>,
    pub target_percent: f64,
}

#[derive(Debug, Clone)]
pub struct AgentEvalDataset {
    pub path: String,
    pub sha256: String,
    pub cases: Vec<EvalCase>,
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub messages: Vec<serde_json::Value>,
    pub tools: Vec<ToolDefinition>,
    pub policy: GenerationPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenUsage {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolExecution {
    pub call_id: String,
    pub tool: String,
    pub arguments: serde_json::Value,
    pub result: serde_json::Value,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ModelCompletion {
    pub content: String,
    pub finish_reason: Option<String>,
    pub usage: TokenUsage,
    pub latency_ms: u64,
    pub timings: Option<serde_json::Value>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OracleCheck {
    pub rule: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalCaseResult {
    pub id: String,
    pub domain: String,
    pub run_index: u32,
    pub effective_seed: i64,
    pub policy: GenerationPolicy,
    pub required_context_tokens: u32,
    pub response: String,
    pub finish_reason: Option<String>,
    pub usage: TokenUsage,
    pub latency_ms: u64,
    pub timings: Vec<serde_json::Value>,
    pub tool_calls: Vec<ToolCall>,
    pub tool_executions: Vec<ToolExecution>,
    pub oracle_checks: Vec<OracleCheck>,
    pub score_percent: Option<f64>,
    pub target_percent: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistributionSummary {
    pub count: usize,
    pub mean: Option<f64>,
    pub stddev: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub ci95_low: Option<f64>,
    pub ci95_high: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PassRateSummary {
    pub count: usize,
    pub passed: usize,
    pub percent: Option<f64>,
    pub wilson95_low: Option<f64>,
    pub wilson95_high: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalUncertainty {
    pub score_percent: DistributionSummary,
    pub latency_ms: DistributionSummary,
    pub pass_rate: PassRateSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentEvalReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub dataset_path: String,
    pub dataset_sha256: String,
    pub endpoint: String,
    pub model: String,
    pub started_at: String,
    pub finished_at: String,
    pub repetitions: u32,
    pub seed_stride: i64,
    pub sample_count: usize,
    pub cases: Vec<EvalCaseResult>,
    pub score_percent: Option<f64>,
    pub uncertainty: EvalUncertainty,
    pub target_percent: f64,
    pub target_confidence_met: bool,
    pub passed: bool,
    pub unknown_count: usize,
    pub residual_risk: Vec<String>,
}
