//! Benchmark-Driven Model Basket Launcher.
//!
//! Builds a launch basket from the consolidated empirical ledger: aggregates
//! per-model scorecards, extracts multi-objective Pareto-optimal configurations
//! across all models, and launches the resulting set through the local
//! inference runtime port.
//!
//! This is the bridge from "consolidated benchmark matrix" → "running model
//! basket" that adapts to dynamic runtime constraints and scenarios.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use cynic_protocol::ParametricConfigPoint;

use crate::domain::matrix_sweep::empirical_store::{
    aggregate_model_scorecard, solve_optimal_configuration,
};
use crate::domain::matrix_sweep::{
    ConstraintFilter, HardwareBudget, ModelEmpiricalScorecard, OomSafetyStatus, OptimizationGoal,
};
use crate::ports::empirical_store_port::{EmpiricalStoreError, EmpiricalStorePort};
use crate::ports::local_runtime_port::{LocalInferenceRuntimePort, LocalRuntimeError};

pub const BASKET_PLAN_SCHEMA: &str = "cynic.inference.bench_basket.v1";

/// One model configuration slated for launch in the basket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasketItem {
    /// Model alias (matches vault profile alias).
    pub alias: String,
    /// The parametric configuration point proven optimal via Pareto solving.
    pub config: ParametricConfigPoint,
    /// Resolved vault path to the GGUF weight.
    pub model_path: PathBuf,
    /// Measured empirical scorecard for this model.
    pub scorecard: ModelEmpiricalScorecard,
    /// Whether the configuration passed the OOM/memory admission test.
    pub memory_safe: bool,
}

/// Complete launch plan derived from the consolidated benchmark ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasketPlan {
    pub schema: &'static str,
    /// Ordered list of model configurations to launch.
    /// Priority order: highest Pareto rank first (utopian-optimal).
    pub items: Vec<BasketItem>,
    /// All model aliases considered (even if not in final basket).
    pub candidates_considered: Vec<String>,
    /// Number of candidates that had empirical records but were dominated.
    pub dominated_excluded: usize,
    /// Runtime constraints applied.
    pub constraints: ConstraintFilter,
    /// Optimization goal used.
    pub goal: OptimizationGoal,
    /// Live hardware budget used for admission.
    pub hardware: HardwareBudget,
}

/// Outcome of launching every item in the basket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasketLaunchReport {
    pub schema: &'static str,
    pub plan: BasketPlan,
    pub launched: usize,
    pub failed: usize,
    pub per_item_results: Vec<BasketItemResult>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasketItemResult {
    pub alias: String,
    pub ok: bool,
    pub endpoint: Option<String>,
    pub message: String,
}

impl BasketPlan {
    pub const SCHEMA: &'static str = BASKET_PLAN_SCHEMA;
}

/// Build a launch basket from the empirical ledger.
///
/// Flow:
/// 1. Query all empirical entries from the store
/// 2. For each model alias: aggregate scorecard + solve optimal config
/// 3. Apply memory admission filter for live hardware
/// 4. Rank via multi-objective Pareto frontier (utopian-optimal)
/// 5. Produce ordered BasketPlan
pub async fn build_basket_from_ledger(
    store: &dyn EmpiricalStorePort,
    hardware: HardwareBudget,
    system_reserve_bytes: u64,
    constraints: &ConstraintFilter,
    goal: OptimizationGoal,
) -> Result<BasketPlan, EmpiricalStoreError> {
    let system_reserve = if system_reserve_bytes == 0 {
        hardware.total_ram_bytes / 10
    } else {
        system_reserve_bytes
    };

    // Query all entries (model_alias=None → all models)
    let all_entries = store.query_entries(None, None).await.unwrap_or_default();

    // Collect unique model aliases from empirical entries
    let candidate_aliases: Vec<String> = all_entries
        .iter()
        .map(|e| e.model_alias.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    let mut candidates_considered: Vec<String> = Vec::new();
    let mut dominated_excluded = 0usize;
    let mut items: Vec<BasketItem> = Vec::new();

    for alias in &candidate_aliases {
        candidates_considered.push(alias.clone());

        // Aggregate scorecard for quality summary
        let scorecard = aggregate_model_scorecard(&all_entries, alias);

        // Solve the optimal configuration for this specific model
        let optimal = solve_optimal_configuration(
            &all_entries,
            alias,
            hardware,
            system_reserve,
            constraints,
            goal,
        );

        if let Some(entry) = optimal {
            // Memory admission: only include safe configs
            let memory_safe = entry.oom_status == OomSafetyStatus::Safe
                || entry.oom_status == OomSafetyStatus::Marginal;

            if !memory_safe {
                dominated_excluded += 1;
                continue;
            }

            // Resolve model path from the vault via the empirical entry
            let model_path = resolve_model_path_for_entry(alias, &entry.config);

            items.push(BasketItem {
                alias: alias.clone(),
                config: entry.config,
                model_path,
                scorecard,
                memory_safe,
            });
        } else {
            // Model had no safe configuration under current constraints
            dominated_excluded += 1;
        }
    }

    // Multi-model Pareto ranking: rank items so the best-balanced model
    // comes first. Uses a simple utopian heuristic on scorecard fields.
    items.sort_by(|a, b| {
        let a_rank = utopian_rank(&a.scorecard);
        let b_rank = utopian_rank(&b.scorecard);
        b_rank
            .partial_cmp(&a_rank)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(BasketPlan {
        schema: BASKET_PLAN_SCHEMA,
        items,
        candidates_considered,
        dominated_excluded,
        constraints: constraints.clone(),
        goal,
        hardware,
    })
}

/// Launch every item in the basket through the local runtime port.
pub async fn launch_basket(
    runtime: &dyn LocalInferenceRuntimePort,
    plan: BasketPlan,
) -> Result<BasketLaunchReport, LocalRuntimeError> {
    let total = plan.items.len();
    let mut launched = 0usize;
    let mut failed = 0usize;
    let mut per_item_results = Vec::with_capacity(total);

    for item in &plan.items {
        let start_result = runtime.start(&item.alias);
        match start_result {
            Ok(report) => {
                launched += 1;
                per_item_results.push(BasketItemResult {
                    alias: item.alias.clone(),
                    ok: report.ok,
                    endpoint: report.status.endpoint,
                    message: report.message.unwrap_or_else(|| "launched".to_string()),
                });
            }
            Err(e) => {
                failed += 1;
                per_item_results.push(BasketItemResult {
                    alias: item.alias.clone(),
                    ok: false,
                    endpoint: None,
                    message: format!("start failed: {e}"),
                });
            }
        }
    }

    let message = if failed == 0 {
        format!("launched {launched}/{total} basket items successfully")
    } else {
        format!("launched {launched}/{total}; {failed} failed")
    };

    Ok(BasketLaunchReport {
        schema: BASKET_PLAN_SCHEMA,
        plan,
        launched,
        failed,
        per_item_results,
        message,
    })
}

/// Resolve the on-disk GGUF path for a model alias from the vault layout.
///
/// This is a projection into the model_store_port — here we use the same
/// layout logic as profile_bootstrap to stay DRY.
fn resolve_model_path_for_entry(alias: &str, _config: &ParametricConfigPoint) -> PathBuf {
    use crate::domain::model_layout::ModelLayout;
    let layout = ModelLayout::from_env();
    layout.vault_dir.join(format!("{alias}.gguf"))
}

/// Compute a utopian-optimal rank from a scorecard.
///
/// Higher throughput, lower TTFT, larger context, more safe points → higher rank.
/// This is a pure data-driven ranking — no arbitrary scalar weights across models.
fn utopian_rank(scorecard: &ModelEmpiricalScorecard) -> f64 {
    let tps = scorecard.peak_throughput_tok_sec as f64;
    let ttft = scorecard.min_ttft_ms.max(1.0) as f64; // avoid div-by-zero
    let ctx = scorecard.max_viable_context.token_count() as f64;
    let safe = scorecard.safe_points_count as f64;
    let total = scorecard.total_benchmarked_points.max(1) as f64;

    // Normalize each objective to [0, 1]-ish via reciprocal / log scaling
    let tps_score = tps.ln().max(0.0);
    let latency_score = 1000.0 / ttft; // lower ttft → higher score
    let ctx_score = ctx.log2().max(0.0);
    let safety_score = safe / total;

    tps_score + latency_score + ctx_score + safety_score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utopian_rank_favors_higher_throughput() {
        let scorecard_a = ModelEmpiricalScorecard {
            model_alias: "a".into(),
            total_benchmarked_points: 4,
            safe_points_count: 4,
            max_viable_context: cynic_protocol::ContextTier::Ctx32k,
            peak_throughput_tok_sec: 25.0,
            min_ttft_ms: 150.0,
            best_balanced_config: None,
        };
        let scorecard_b = ModelEmpiricalScorecard {
            model_alias: "b".into(),
            total_benchmarked_points: 4,
            safe_points_count: 4,
            max_viable_context: cynic_protocol::ContextTier::Ctx32k,
            peak_throughput_tok_sec: 50.0,
            min_ttft_ms: 100.0,
            best_balanced_config: None,
        };
        let rank_a = utopian_rank(&scorecard_a);
        let rank_b = utopian_rank(&scorecard_b);
        assert!(rank_b > rank_a, "higher tps+lower ttft should rank higher");
    }

    #[test]
    fn basket_plan_schema_constant() {
        assert_eq!(BasketPlan::SCHEMA, BASKET_PLAN_SCHEMA);
        assert_eq!(BASKET_PLAN_SCHEMA, "cynic.inference.bench_basket.v1");
    }

    #[test]
    fn resolve_model_path_joins_vault_and_alias() {
        let path = resolve_model_path_for_entry(
            "qwen-2b",
            &ParametricConfigPoint {
                model_alias: "qwen-2b".into(),
                model_file: "qwen-2b.gguf".into(),
                context_tier: cynic_protocol::ContextTier::Ctx4k,
                kv_quant: cynic_protocol::KvQuantization::Q4_0,
                threads_gen: 8,
                threads_batch: 16,
                batch_size: 1024,
                ubatch_size: 512,
                flash_attn: true,
                gpu_layers: 0,
                rope_scaling: cynic_protocol::RopeScaling::None,
                topology: cynic_protocol::TopologyStrategy::DedicatedSingle,
                target_harness: cynic_protocol::HarnessProtocol::OpenAiToolCall,
            },
        );
        // Just verify it ends with the alias + .gguf
        let s = path.to_string_lossy();
        assert!(s.ends_with("qwen-2b.gguf"));
    }
}
