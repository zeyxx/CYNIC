//! Native CYNIC benchmark policy & plan generator.
//!
//! Loads `infra/benchmark-kv-sweep-policy.json` and produces canonical
//! `BenchmarkPlan` instances directly in Rust without external scripts.

use std::path::Path;
use serde::{Deserialize, Serialize};

use super::plan::BenchmarkPlan;
use super::types::{ModelParameters, ModelSpec, ScenarioSpec, TaskKind, TaskSpec, Thresholds};
use crate::domain::local_runtime::LocalModelProfile;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcceptanceBaselineSpec {
    pub min_context_baseline: u32,
    #[serde(default)]
    pub extended_contexts: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UseCaseSpec {
    pub id: String,
    pub priority: u8,
    pub description: String,
    #[serde(default)]
    pub target_metrics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KvCacheSweepSpec {
    pub standard_types: Vec<String>,
    #[serde(default)]
    pub micro_model_types: Vec<String>,
    #[serde(default)]
    pub micro_model_threshold_b: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionParameterSweepSpec {
    #[serde(default)]
    pub batch_sizes: Vec<u32>,
    #[serde(default)]
    pub ubatch_sizes: Vec<u32>,
    #[serde(default)]
    pub thread_counts: Vec<u16>,
    #[serde(default)]
    pub temperatures: Vec<f32>,
    #[serde(default)]
    pub thinking_modes: Vec<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkPolicySpec {
    pub schema: String,
    pub policy_id: String,
    pub description: String,
    pub acceptance_baseline: AcceptanceBaselineSpec,
    #[serde(default)]
    pub use_cases: Vec<UseCaseSpec>,
    pub kv_cache_sweep: KvCacheSweepSpec,
    pub execution_parameter_sweep: ExecutionParameterSweepSpec,
}

pub fn load_benchmark_policy(path: &Path) -> Result<BenchmarkPolicySpec, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read policy file {}: {err}", path.display()))?;
    let policy: BenchmarkPolicySpec = serde_json::from_str(&raw)
        .map_err(|err| format!("failed to parse policy JSON {}: {err}", path.display()))?;
    Ok(policy)
}

/// Generate a native `BenchmarkPlan` for a set of model profiles given a `BenchmarkPolicySpec`.
pub fn generate_plan_from_policy(
    policy: &BenchmarkPolicySpec,
    profiles: &[LocalModelProfile],
) -> BenchmarkPlan {
    let mut scenarios = Vec::new();

    let contexts = if policy.acceptance_baseline.extended_contexts.is_empty() {
        vec![policy.acceptance_baseline.min_context_baseline]
    } else {
        let mut ctxs = vec![policy.acceptance_baseline.min_context_baseline];
        for &c in &policy.acceptance_baseline.extended_contexts {
            if c != policy.acceptance_baseline.min_context_baseline {
                ctxs.push(c);
            }
        }
        ctxs
    };

    let batch = policy
        .execution_parameter_sweep
        .batch_sizes
        .first()
        .copied()
        .unwrap_or(2048);
    let ubatch = policy
        .execution_parameter_sweep
        .ubatch_sizes
        .first()
        .copied()
        .unwrap_or(512);
    let threads = policy
        .execution_parameter_sweep
        .thread_counts
        .first()
        .copied()
        .unwrap_or(8);

    for profile in profiles {
        if !profile.model_exists {
            continue;
        }

        let kv_types = &policy.kv_cache_sweep.standard_types;

        for &ctx in &contexts {
            for kv_type in kv_types {
                let scenario_id = format!("{}-{}-ctx{}-kv{}", profile.alias, policy.policy_id, ctx / 1024, kv_type);

                let spec = ScenarioSpec {
                    scenario_id: scenario_id.clone(),
                    correlation_id: Some(scenario_id.clone()),
                    coordination_id: None,
                    probe_runtime_plan: false,
                    runtime_kind: super::types::RuntimeKind::DirectLlama,
                    runtime_id: Some(profile.alias.clone()),
                    runtime_version: None,
                    coordination_kind: None,
                    tool_count: None,
                    max_agent_steps: None,
                    model: ModelSpec {
                        role: "primary".to_string(),
                        parameters: ModelParameters {
                            model_id: profile.alias.clone(),
                            model_path: profile.model_path.clone(),
                            model_digest: None,
                            architecture: None,
                            quantization: None,
                            context_requested: ctx,
                            parallel_slots: 1,
                            context_per_slot: Some(ctx),
                            kv_cache_type_k: Some(kv_type.clone()),
                            kv_cache_type_v: Some(kv_type.clone()),
                            flash_attention: Some(true),
                            gpu_layers: Some(99),
                            fit_mode: None,
                            threads: Some(threads),
                            batch_size: Some(batch),
                            ubatch_size: Some(ubatch),
                            temperature: Some(0.7),
                            top_p: Some(0.95),
                            reasoning_budget: None,
                        },
                    },
                    tasks: vec![TaskSpec {
                        task_id: format!("kv-sweep-{}", kv_type),
                        kind: TaskKind::PromptOnly,
                        prompt: "CYNIC native benchmark sweep verification prompt.".to_string(),
                        max_output_tokens: 128,
                        expected_proof: "ok".to_string(),
                    }],
                    thresholds: Thresholds {
                        min_available_memory_bytes: None,
                        min_available_memory_percent: Some(5.0),
                        max_rss_bytes: None,
                        max_rss_percent: Some(95.0),
                        max_wall_time_ms: Some(300_000),
                    },
                };
                scenarios.push(spec);
            }
        }
    }

    BenchmarkPlan::new(format!("{}-generated", policy.policy_id), scenarios)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_policy_and_generate_plan() {
        let policy_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../infra/benchmark-kv-sweep-policy.json");
        let policy = load_benchmark_policy(&policy_path).expect("should load policy");
        assert_eq!(policy.acceptance_baseline.min_context_baseline, 65536);

        let mock_profile = LocalModelProfile {
            alias: "qwythos-9b-q4".to_string(),
            model_path: "/tmp/fake.gguf".to_string(),
            source: crate::domain::local_runtime::LocalModelSource::Vault,
            model_exists: true,
            size_bytes: Some(1000),
            host: "127.0.0.1".to_string(),
            port: 8080,
            ctx_size: 32768,
            extra_args: "".to_string(),
            memory_max: None,
            cpu_quota: None,
            loopback_only: true,
        };

        let plan = generate_plan_from_policy(&policy, &[mock_profile]);
        assert!(!plan.scenarios.is_empty());
        assert_eq!(plan.plan_id, "cynic-kv-sweep-policy-v1-generated");
        let first_scenario = &plan.scenarios[0].scenario;
        assert_eq!(first_scenario.model.parameters.context_requested, 65536);
    }
}
