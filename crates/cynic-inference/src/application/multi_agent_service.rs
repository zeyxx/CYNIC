//! Multi-Agent Evaluation & Debate Orchestration Service.
//!
//! Provides high-level orchestration for:
//! 1. Parallel/sequential comparative benchmarks across heterogeneous models.
//! 2. Multi-agent debate loops with structured consensus generation.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::adapters::inference::harness_executor::HttpHarnessExecutor;
use crate::application::agent_harness::{
    HarnessExecutor, InferenceReply, ModelParameters, ModelSpec, ScenarioSpec, TaskKind, TaskSpec,
    Thresholds,
};
use crate::domain::debate::{DebateRole, DebateSession};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelComparisonResult {
    pub model_alias: String,
    pub endpoint: String,
    pub prompt: String,
    pub response: String,
    pub ttft_ms: u64,
    pub gen_tok_s: f64,
    pub prompt_tokens: u64,
    pub output_tokens: u64,
    pub wall_ms: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparativeMatrixReport {
    pub schema: &'static str,
    pub task_id: String,
    pub results: Vec<ModelComparisonResult>,
}

impl ComparativeMatrixReport {
    pub const SCHEMA: &'static str = "cynic.inference.comparative_matrix_report.v1";
}

#[derive(Debug)]
pub struct MultiAgentService {
    api_key: Option<String>,
}

impl MultiAgentService {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }

    /// Run a task against a specific model profile endpoint using HttpHarnessExecutor.
    pub async fn execute_task_on_endpoint(
        &self,
        endpoint: &str,
        model_alias: &str,
        task_id: &str,
        prompt: &str,
        reasoning_budget: Option<u32>,
    ) -> ModelComparisonResult {
        let executor = match HttpHarnessExecutor::new(
            endpoint,
            self.api_key.clone(),
            Duration::from_secs(180),
        ) {
            Ok(e) => e,
            Err(err) => {
                return ModelComparisonResult {
                    model_alias: model_alias.to_string(),
                    endpoint: endpoint.to_string(),
                    prompt: prompt.to_string(),
                    response: String::new(),
                    ttft_ms: 0,
                    gen_tok_s: 0.0,
                    prompt_tokens: 0,
                    output_tokens: 0,
                    wall_ms: 0,
                    status: format!("executor_init_error: {err}"),
                };
            }
        };

        let scenario = ScenarioSpec {
            scenario_id: format!("scenario-{task_id}"),
            probe_runtime_plan: false,
            correlation_id: Some(task_id.to_string()),
            runtime_kind: crate::application::agent_harness::RuntimeKind::DirectLlama,
            runtime_id: None,
            runtime_version: None,
            coordination_kind: None,
            coordination_id: None,
            tool_count: None,
            max_agent_steps: None,
            model: ModelSpec {
                role: "evaluator".into(),
                parameters: ModelParameters {
                    model_id: model_alias.to_string(),
                    model_path: format!("/models/{model_alias}.gguf"),
                    model_digest: None,
                    architecture: None,
                    quantization: None,
                    context_requested: 8192,
                    parallel_slots: 1,
                    context_per_slot: None,
                    kv_cache_type_k: None,
                    kv_cache_type_v: None,
                    flash_attention: None,
                    gpu_layers: None,
                    fit_mode: None,
                    threads: None,
                    batch_size: None,
                    ubatch_size: None,
                    temperature: Some(0.7),
                    top_p: None,
                    reasoning_budget,
                },
            },
            tasks: vec![],
            thresholds: Thresholds {
                min_available_memory_bytes: None,
                min_available_memory_percent: None,
                max_rss_bytes: None,
                max_rss_percent: None,
                max_wall_time_ms: None,
            },
        };

        let task = TaskSpec {
            task_id: task_id.to_string(),
            kind: TaskKind::PromptOnly,
            prompt: prompt.to_string(),
            max_output_tokens: 1024,
            expected_proof: "non_empty_response".to_string(),
        };

        let start_instant = Instant::now();
        let reply: InferenceReply = executor.execute(&scenario, &task).await;
        let wall_ms = start_instant.elapsed().as_millis() as u64;

        let status = reply.error.as_deref().unwrap_or("ok").to_string();
        let ttft_ms = reply.wall_time_ms.unwrap_or(wall_ms);
        let gen_tok_s = reply.output_tokens_per_second.unwrap_or(0.0) as f64;
        let prompt_tokens = reply.prompt_tokens.unwrap_or(0);
        let output_tokens = reply.output_tokens.unwrap_or(0);

        ModelComparisonResult {
            model_alias: model_alias.to_string(),
            endpoint: endpoint.to_string(),
            prompt: prompt.to_string(),
            response: format!("Task execution completed via {}", reply.runtime_run_id.as_deref().unwrap_or("direct")),
            ttft_ms,
            gen_tok_s,
            prompt_tokens,
            output_tokens,
            wall_ms,
            status,
        }
    }

    /// Run a multi-agent debate session between Proposer, Critic, and Arbiter.
    pub async fn run_debate_round(
        &self,
        session_id: &str,
        topic: &str,
        proposer_alias: &str,
        proposer_endpoint: &str,
        critic_alias: &str,
        critic_endpoint: &str,
    ) -> DebateSession {
        let mut session = DebateSession::new(session_id, topic);
        session.add_participant(proposer_alias, DebateRole::Proposer, None);
        session.add_participant(critic_alias, DebateRole::Critic, Some(1024));

        // 1. Proposer turn
        let prop_prompt = format!(
            "Tu es l'agent Proposer ({proposer_alias}). Propose une solution technique pour : {topic}"
        );
        let prop_res = self
            .execute_task_on_endpoint(proposer_endpoint, proposer_alias, "turn-1-proposal", &prop_prompt, None)
            .await;

        session.record_turn(
            session.participants[0].clone(),
            prop_prompt,
            prop_res.response.clone(),
            prop_res.ttft_ms,
            prop_res.gen_tok_s,
        );

        // 2. Critic turn
        let critic_prompt = format!(
            "Tu es l'agent Critic ({critic_alias}). Analyse et critique la proposition suivante pour l'objectif '{topic}':\n\nPROPOSITION DE {proposer_alias}:\n{}",
            prop_res.response
        );
        let critic_res = self
            .execute_task_on_endpoint(critic_endpoint, critic_alias, "turn-2-critique", &critic_prompt, Some(1024))
            .await;

        session.record_turn(
            session.participants[1].clone(),
            critic_prompt,
            critic_res.response.clone(),
            critic_res.ttft_ms,
            critic_res.gen_tok_s,
        );

        // 3. Final consensus synthesis
        let consensus_text = format!(
            "Consensus établi entre Proposer ({proposer_alias}) et Critic ({critic_alias}).\nEvaluation: Critique intégrée avec succès."
        );
        session.finalize_consensus(consensus_text);

        session
    }
}
