//! LLM port — injectable language model interface for agents.
//!
//! The LLM is never the authority. It is a tool the agent calls when
//! pure reconciliation cannot resolve a decision. The agent owns the
//! budget, the escalation threshold, and the fail-closed behavior.
//!
//! Three implementations are provided:
//! - `DirectLlmPort`: external API (OpenRouter, Groq, etc.)
//! - `HermesAgentPort`: Hermes harness on CYNIC inference endpoint
//! - `NativeRulePort`: no LLM, pure Rust rules (fast, sovereign, free)

use serde::{Deserialize, Serialize};

/// Schema for LLM capability contracts.
pub const LLM_CAPABILITY_SCHEMA: &str = "cynic.llm_capability.v1";
pub const LLM_REQUEST_SCHEMA: &str = "cynic.llm_request.v1";
pub const LLM_RESPONSE_SCHEMA: &str = "cynic.llm_response.v1";

/// What the LLM is being asked to do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmTask {
    /// Resolve a git merge conflict in a file.
    MergeConflict {
        file_path: String,
        conflict_hunk: String,
        source_branch: String,
        target_branch: String,
    },
    /// Summarize observation data for human review.
    SummarizeObservation { observation_json: String },
    /// Propose a command to run next given current state.
    ProposeAction {
        current_state: String,
        available_commands: Vec<String>,
    },
    /// Classify the severity of an observation.
    ClassifySeverity { observation_json: String },
}

/// A request to the LLM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmRequest {
    pub schema: String,
    pub task: LlmTask,
    /// Context window: what the LLM needs to know.
    pub context: String,
    /// Expected output format hint.
    pub output_format: String,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Temperature for sampling.
    pub temperature: f32,
}

/// The LLM's response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmResponse {
    pub schema: String,
    /// Raw text output from the LLM.
    pub output: String,
    /// Whether the response was truncated.
    pub truncated: bool,
    /// Tokens consumed (if available).
    pub tokens_used: Option<u32>,
    /// Model that generated this response.
    pub model: String,
}

/// Port trait for LLM implementations.
///
/// Agents call this port. The port never decides — it only generates
/// text. The agent decides whether to use the text, escalate, or retry.
pub trait LlmPort: Send + Sync {
    /// Name of this LLM implementation (for logging/audit).
    fn name(&self) -> &'static str;

    /// Whether this LLM is currently available.
    fn is_available(&self) -> bool;

    /// Execute an LLM request and return the response.
    ///
    /// Returns `None` if the LLM is unavailable or the request failed.
    /// The agent must handle `None` gracefully (fail-closed).
    fn complete(&self, request: &LlmRequest) -> Option<LlmResponse>;

    /// Estimate the cost of a request (in tokens or $).
    fn estimate_cost(&self, request: &LlmRequest) -> LlmCostEstimate;
}

/// Cost estimate for an LLM request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmCostEstimate {
    pub tokens_estimate: u32,
    pub cost_usd_estimate: f64,
    pub energy_joules_estimate: f64,
}

/// Port for agents that operate CYNIC via CLI binaries.
///
/// This is the "operator agent" pattern: the agent observes the organism
/// state via CLI, asks the LLM what to do, executes the chosen command,
/// and produces a proof.
pub trait OperatorAgentPort: Send + Sync {
    /// Name of this operator agent.
    fn agent_name(&self) -> &'static str;

    /// Observe the current state of the organism.
    fn observe(&self) -> OperatorObservation;

    /// Ask the LLM what action to propose given the observation.
    fn propose_action(
        &self,
        observation: &OperatorObservation,
        llm: &dyn LlmPort,
    ) -> Option<OperatorAction>;

    /// Execute a proposed action.
    fn execute_action(&self, action: &OperatorAction) -> OperatorProof;

    /// Whether this agent can handle the given observation.
    fn can_handle(&self, observation: &OperatorObservation) -> bool;
}

/// Observation of the organism state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorObservation {
    pub schema: String,
    /// Current branch.
    pub current_branch: String,
    /// Git status (short format).
    pub git_status: String,
    /// Recent commits (last 5).
    pub recent_commits: Vec<String>,
    /// Available CYNIC commands.
    pub available_commands: Vec<String>,
    /// Any active claims.
    pub active_claims: Vec<String>,
}

/// Proposed action from the LLM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorAction {
    pub schema: String,
    /// The command to execute.
    pub command: String,
    /// Arguments to pass.
    pub args: Vec<String>,
    /// Human-readable rationale.
    pub rationale: String,
    /// Confidence in this action (0.0 = uncertain, 1.0 = certain).
    pub confidence: f64,
    /// Whether this action requires human approval.
    pub requires_approval: bool,
}

/// Proof of action execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorProof {
    pub schema: String,
    pub action: OperatorAction,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    /// SHA256 of the resulting state (if applicable).
    pub state_hash: Option<String>,
}
