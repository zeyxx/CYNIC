//! Empirical Performance Store & Closed-Loop Constraint Solver.
//!
//! Ground truth for model execution profiles derived from measured telemetry,
//! not static human guesswork.
//!
//! Replaces static `.toml` declarations with empirical scorecards that map
//! `(Model, Hardware, Context, Quantization, Threads)` -> Measured Performance (TTFT, tok/s, RSS, Jitter).
//!
//! Selection policy is strictly data-driven via Pareto dominance — no arbitrary
//! scalar weight coefficients.

use cynic_protocol::{ContextTier, ParametricConfigPoint};
use serde::{Deserialize, Serialize};

use super::memory_estimator::can_admit_model_configuration;
use super::types::{HardwareBudget, StreamDynamicsReport};
use crate::math::pareto::{ObjectiveVector, extract_pareto_frontier, select_utopian_optimal};

pub const EMPIRICAL_LEDGER_SCHEMA: &str = "cynic.inference.empirical_ledger.v1";

/// OOM / Memory Safety Assessment Status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OomSafetyStatus {
    /// Safe: measured RSS and KV allocation stay well within hardware limits.
    Safe,
    /// Marginal: fits within RAM, but buffer < 10% of total memory.
    Marginal,
    /// OOM: process was terminated or allocation failed during execution.
    Oom,
    /// Execution error (e.g. invalid context size, unsupported RoPE).
    ExecutionError,
}

/// Provenance metadata describing the execution environment where the telemetry was captured.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkProvenance {
    pub timestamp_unix_s: u64,
    pub total_ram_bytes: u64,
    pub available_ram_bytes_at_start: u64,
    pub cpu_arch: String,
    pub os_kernel: String,
}

/// A single immutable empirical performance record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmpiricalProfileEntry {
    /// Cryptographic or semantic model identity.
    pub model_alias: String,
    pub model_sha256: Option<String>,
    pub model_weight_bytes: u64,

    /// Parametric configuration point tested.
    pub config: ParametricConfigPoint,

    /// Measured streaming dynamics.
    pub stream_dynamics: StreamDynamicsReport,

    /// Peak Resident Set Size (RSS) observed in physical RAM.
    pub rss_peak_bytes: u64,

    /// Optional measured inputs for a reproducible physical Roofline bound.
    /// Absent means the bound is unknown, never estimated from device identity.
    #[serde(default)]
    pub roofline_inputs: Option<crate::math::roofline::RooflineInputs>,

    /// Domain quality score (0.0 to 1.0) derived from scenario assertions.
    pub quality_score: Option<f32>,

    /// Memory safety assessment.
    pub oom_status: OomSafetyStatus,

    /// Provenance of the run.
    pub provenance: BenchmarkProvenance,
}

/// Optimization goals for the constraint solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationGoal {
    /// Maximize token generation throughput (tokens / second).
    MaxThroughput,
    /// Minimize Time-To-First-Token latency (interactive agents).
    MinLatency,
    /// Minimize inter-token jitter variance (smooth streaming UI / voice).
    MinJitter,
    /// Maximize context length while respecting latency and safety bounds.
    MaxContext,
    /// Pareto point with the smallest maximum normalized regret across observed
    /// dimensions. This is an explicit equal-regret default, not a fixed weight table.
    Balanced,
}

/// Hard constraints passed to the constraint solver.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConstraintFilter {
    /// Required minimum context tier (e.g. at least 32k or 64k).
    pub min_context: Option<ContextTier>,
    /// Maximum allowable Time-To-First-Token in milliseconds.
    pub max_ttft_ms: Option<f32>,
    /// Minimum required generation speed (tokens / second).
    pub min_gen_tokens_per_sec: Option<f32>,
    /// Maximum allowed physical RAM footprint in bytes.
    pub max_ram_bytes: Option<u64>,
    /// Require Flash Attention to be enabled.
    pub require_flash_attn: bool,
    /// Require GPU layer offloading (> 0).
    pub require_gpu_offload: bool,
}

/// Summary scorecard aggregating performance for a model across its valid configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEmpiricalScorecard {
    pub model_alias: String,
    pub total_benchmarked_points: usize,
    pub safe_points_count: usize,
    pub max_viable_context: ContextTier,
    pub peak_throughput_tok_sec: f32,
    pub min_ttft_ms: f32,
    pub best_balanced_config: Option<ParametricConfigPoint>,
}

/// Pure constraint solver: selects the mathematically optimal execution vector from empirical records.
pub fn solve_optimal_configuration(
    records: &[EmpiricalProfileEntry],
    model_alias: &str,
    live_hardware: HardwareBudget,
    system_reserve_bytes: u64,
    constraints: &ConstraintFilter,
    goal: OptimizationGoal,
) -> Option<EmpiricalProfileEntry> {
    let candidates: Vec<&EmpiricalProfileEntry> = records
        .iter()
        .filter(|entry| entry.model_alias.eq_ignore_ascii_case(model_alias))
        // Rule 1: Exclude non-safe runs (OOM / execution errors)
        .filter(|entry| {
            entry.oom_status == OomSafetyStatus::Safe
                || entry.oom_status == OomSafetyStatus::Marginal
        })
        // Rule 2: Exclude configurations that would exceed live available RAM or explicit user cap
        .filter(|entry| {
            if let Some(user_max_ram) = constraints.max_ram_bytes {
                if entry.rss_peak_bytes > user_max_ram {
                    return false;
                }
            }
            can_admit_model_configuration(
                entry.rss_peak_bytes,
                live_hardware.available_ram_bytes,
                system_reserve_bytes,
            )
        })
        // Rule 3: Context tier minimum
        .filter(|entry| {
            if let Some(min_ctx) = constraints.min_context {
                entry.config.context_tier.token_count() >= min_ctx.token_count()
            } else {
                true
            }
        })
        // Rule 4: Max TTFT SLA
        .filter(|entry| {
            if let Some(max_ttft) = constraints.max_ttft_ms {
                entry.stream_dynamics.ttft_ms <= max_ttft
            } else {
                true
            }
        })
        // Rule 5: Min generation throughput
        .filter(|entry| {
            if let Some(min_tps) = constraints.min_gen_tokens_per_sec {
                entry.stream_dynamics.predicted_tokens_per_second >= min_tps
            } else {
                true
            }
        })
        // Rule 6: Hardware requirements
        .filter(|entry| {
            if constraints.require_flash_attn && !entry.config.flash_attn {
                return false;
            }
            if constraints.require_gpu_offload && entry.config.gpu_layers == 0 {
                return false;
            }
            true
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Build Pareto frontier from the constraint-filtered candidate set.
    // Only configurations that no other configuration strictly dominates on all 5 objectives survive.
    // Deref &&T -> &T so select_from_frontier and to_objective_vector receive &EmpiricalProfileEntry.
    let frontier_owned: Vec<&EmpiricalProfileEntry> =
        extract_pareto_frontier(&candidates, |e| to_objective_vector(e))
            .into_iter()
            .copied()
            .collect();

    // From the non-dominated frontier, select according to the agent's goal.
    select_from_frontier(&frontier_owned, goal).cloned()
}

/// Maps an empirical record to its measured objective vector. Missing quality
/// remains explicit rather than being replaced by a synthetic score.
fn to_objective_vector(entry: &EmpiricalProfileEntry) -> ObjectiveVector {
    ObjectiveVector {
        throughput_tps: entry.stream_dynamics.predicted_tokens_per_second,
        ttft_ms: entry.stream_dynamics.ttft_ms,
        jitter_variance_ms: entry.stream_dynamics.jitter_variance_ms,
        context_tokens: entry.config.context_tier.token_count(),
        quality_score: entry.quality_score,
    }
}

/// Projects a stated goal onto the observed Pareto frontier. `Balanced` uses
/// the frontier's data-normalized equal-regret projection; it does not apply
/// fixed scalar weights.
fn select_from_frontier<'a>(
    frontier: &'a [&'a EmpiricalProfileEntry],
    goal: OptimizationGoal,
) -> Option<&'a EmpiricalProfileEntry> {
    match goal {
        OptimizationGoal::MaxThroughput => frontier
            .iter()
            .max_by(|a, b| {
                a.stream_dynamics
                    .predicted_tokens_per_second
                    .partial_cmp(&b.stream_dynamics.predicted_tokens_per_second)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied(),
        OptimizationGoal::MinLatency => frontier
            .iter()
            .min_by(|a, b| {
                a.stream_dynamics
                    .ttft_ms
                    .partial_cmp(&b.stream_dynamics.ttft_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied(),
        OptimizationGoal::MinJitter => frontier
            .iter()
            .min_by(|a, b| {
                a.stream_dynamics
                    .jitter_variance_ms
                    .partial_cmp(&b.stream_dynamics.jitter_variance_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied(),
        OptimizationGoal::MaxContext => frontier
            .iter()
            .max_by_key(|e| e.config.context_tier.token_count())
            .copied(),
        OptimizationGoal::Balanced => select_utopian_optimal(frontier, to_objective_vector),
    }
}

/// Aggregates an empirical scorecard across all benchmarked points for a model.
pub fn aggregate_model_scorecard(
    records: &[EmpiricalProfileEntry],
    model_alias: &str,
) -> ModelEmpiricalScorecard {
    let model_records: Vec<&EmpiricalProfileEntry> = records
        .iter()
        .filter(|r| r.model_alias.eq_ignore_ascii_case(model_alias))
        .collect();

    let viable_records: Vec<&EmpiricalProfileEntry> = model_records
        .iter()
        .filter(|r| {
            r.oom_status == OomSafetyStatus::Safe || r.oom_status == OomSafetyStatus::Marginal
        })
        .copied()
        .collect();

    let total = model_records.len();
    let safe_count = model_records
        .iter()
        .filter(|r| r.oom_status == OomSafetyStatus::Safe)
        .count();

    let max_viable_context = viable_records
        .iter()
        .map(|r| r.config.context_tier)
        .max_by_key(|c| c.token_count())
        .unwrap_or(ContextTier::Ctx4k);

    let peak_throughput = viable_records
        .iter()
        .map(|r| r.stream_dynamics.predicted_tokens_per_second)
        .fold(0.0_f32, f32::max);

    let min_ttft = viable_records
        .iter()
        .map(|r| r.stream_dynamics.ttft_ms)
        .fold(f32::MAX, f32::min);

    // Pareto utopian: extract the non-dominated frontier then select the point
    // closest to the ideal 5D utopian bound. No arbitrary weights.
    let frontier: Vec<&EmpiricalProfileEntry> =
        extract_pareto_frontier(&viable_records, |e| to_objective_vector(e))
            .into_iter()
            .copied()
            .collect();
    let best_balanced =
        select_utopian_optimal(&frontier, to_objective_vector).map(|r| r.config.clone());

    ModelEmpiricalScorecard {
        model_alias: model_alias.to_string(),
        total_benchmarked_points: total,
        safe_points_count: safe_count,
        max_viable_context,
        peak_throughput_tok_sec: peak_throughput,
        min_ttft_ms: if min_ttft == f32::MAX { 0.0 } else { min_ttft },
        best_balanced_config: best_balanced,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{HarnessProtocol, KvQuantization, RopeScaling, TopologyStrategy};

    fn make_test_entry(
        model: &str,
        ctx: ContextTier,
        tps: f32,
        ttft: f32,
        rss: u64,
        oom: OomSafetyStatus,
    ) -> EmpiricalProfileEntry {
        EmpiricalProfileEntry {
            model_alias: model.into(),
            model_sha256: Some("sha256_mock".into()),
            model_weight_bytes: 4 * 1024 * 1024 * 1024,
            config: ParametricConfigPoint {
                model_alias: model.into(),
                model_file: format!("{model}.gguf"),
                context_tier: ctx,
                kv_quant: KvQuantization::Q4_0,
                threads_gen: 8,
                threads_batch: 16,
                batch_size: 1024,
                ubatch_size: 512,
                flash_attn: true,
                gpu_layers: 24,
                rope_scaling: RopeScaling::None,
                topology: TopologyStrategy::DedicatedSingle,
                target_harness: HarnessProtocol::OpenAiToolCall,
            },
            stream_dynamics: StreamDynamicsReport {
                ttft_ms: ttft,
                mean_inter_token_ms: 1000.0 / tps,
                jitter_variance_ms: 2.0,
                predicted_tokens_per_second: tps,
                prompt_tokens_per_second: 150.0,
            },
            rss_peak_bytes: rss,
            roofline_inputs: None,
            quality_score: Some(0.95),
            oom_status: oom,
            provenance: BenchmarkProvenance {
                timestamp_unix_s: 1786830000,
                total_ram_bytes: 32 * 1024 * 1024 * 1024,
                available_ram_bytes_at_start: 24 * 1024 * 1024 * 1024,
                cpu_arch: "x86_64".into(),
                os_kernel: "Linux".into(),
            },
        }
    }

    #[test]
    fn solver_selects_fastest_safe_configuration() {
        let e1 = make_test_entry(
            "qwen-2b",
            ContextTier::Ctx4k,
            25.0,
            150.0,
            3 * 1024 * 1024 * 1024,
            OomSafetyStatus::Safe,
        );
        let e2 = make_test_entry(
            "qwen-2b",
            ContextTier::Ctx64k,
            18.0,
            350.0,
            8 * 1024 * 1024 * 1024,
            OomSafetyStatus::Safe,
        );
        let e3_oom = make_test_entry(
            "qwen-2b",
            ContextTier::Ctx128k,
            30.0,
            50.0,
            35 * 1024 * 1024 * 1024,
            OomSafetyStatus::Oom,
        );

        let records = vec![e1.clone(), e2.clone(), e3_oom];

        let hw = HardwareBudget {
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            available_ram_bytes: 24 * 1024 * 1024 * 1024,
            gpu_vram_bytes: 8 * 1024 * 1024 * 1024,
        };

        // Goal: MaxThroughput without context filter -> should pick e1 (25 tps > 18 tps)
        let optimal = solve_optimal_configuration(
            &records,
            "qwen-2b",
            hw,
            2 * 1024 * 1024 * 1024,
            &ConstraintFilter::default(),
            OptimizationGoal::MaxThroughput,
        );

        assert!(optimal.is_some());
        assert_eq!(optimal.unwrap().config.context_tier, ContextTier::Ctx4k);

        // Goal: MaxContext with 64k constraint -> should pick e2
        let optimal_64k = solve_optimal_configuration(
            &records,
            "qwen-2b",
            hw,
            2 * 1024 * 1024 * 1024,
            &ConstraintFilter {
                min_context: Some(ContextTier::Ctx64k),
                ..Default::default()
            },
            OptimizationGoal::MaxThroughput,
        );

        assert!(optimal_64k.is_some());
        assert_eq!(
            optimal_64k.unwrap().config.context_tier,
            ContextTier::Ctx64k
        );
    }

    #[test]
    fn scorecard_computes_accurate_aggregates() {
        let e1 = make_test_entry(
            "qwen-2b",
            ContextTier::Ctx4k,
            25.0,
            150.0,
            3 * 1024 * 1024 * 1024,
            OomSafetyStatus::Safe,
        );
        let e2 = make_test_entry(
            "qwen-2b",
            ContextTier::Ctx64k,
            18.0,
            350.0,
            8 * 1024 * 1024 * 1024,
            OomSafetyStatus::Safe,
        );

        let records = vec![e1, e2];
        let scorecard = aggregate_model_scorecard(&records, "qwen-2b");

        assert_eq!(scorecard.total_benchmarked_points, 2);
        assert_eq!(scorecard.safe_points_count, 2);
        assert_eq!(scorecard.max_viable_context, ContextTier::Ctx64k);
        assert_eq!(scorecard.peak_throughput_tok_sec, 25.0);
        assert_eq!(scorecard.min_ttft_ms, 150.0);
        assert!(scorecard.best_balanced_config.is_some());
    }

    #[test]
    fn legacy_ledger_record_keeps_roofline_unknown() {
        let entry = make_test_entry(
            "qwen-2b",
            ContextTier::Ctx4k,
            25.0,
            150.0,
            3 * 1024 * 1024 * 1024,
            OomSafetyStatus::Safe,
        );
        let mut legacy_json = serde_json::to_value(entry).expect("serialize test record");
        legacy_json
            .as_object_mut()
            .expect("object record")
            .remove("roofline_inputs");

        let decoded: EmpiricalProfileEntry =
            serde_json::from_value(legacy_json).expect("legacy records deserialize");
        assert_eq!(decoded.roofline_inputs, None);
    }
}
